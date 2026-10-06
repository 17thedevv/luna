use crate::registry::{CanonicalSymbolId, ExternalSymbol, ModuleRegistry, ProviderInterface};
use luna_borrowck::effect::{
    CallEffectSummary, RawPointerAnchorReturnEffect, RawPointerAnchorSource, RawPointerReturnEffect,
};
use luna_llib::metadata::{
    CanonicalAssociatedEquality, CanonicalInterface, CanonicalLifetime, CanonicalRawPointerAnchor,
    CanonicalRawPointerAnchorSource, CanonicalRawPointerEffect, CanonicalRawPointerEffects,
    CanonicalRawPointerOrigin, CanonicalTraitBound, CanonicalType, ExportedSymbol,
    GenericConstraints, ImplHeader, SemanticMetadata, StableSymbolId, TraitDefinition,
    CanonicalNominalLayout, CanonicalNominalMembers, CanonicalNominalField, CanonicalNominalVariant,
};
use luna_mvir::GlobalId;
use luna_semantic::ty::{SemanticType, SemanticTypeId};
use std::collections::HashMap;

pub struct MetadataBuilder<'a> {
    registry: &'a ModuleRegistry,
    provider: &'a ProviderInterface,
    type_map: HashMap<SemanticTypeId, u32>,
    canonical_types: Vec<CanonicalType>,
    canonical_type_indices: HashMap<CanonicalType, u32>,
    scoped_symbols: HashMap<CanonicalSymbolId, StableSymbolId>,
    nominal_queue: std::collections::BTreeMap<StableSymbolId, luna_common::ids::SymbolId>,
    raw_summaries: &'a HashMap<GlobalId, CallEffectSummary>,
}

impl<'a> MetadataBuilder<'a> {
    pub fn new(
        registry: &'a ModuleRegistry,
        provider: &'a ProviderInterface,
        raw_summaries: &'a HashMap<GlobalId, CallEffectSummary>,
    ) -> Self {
        Self {
            registry,
            provider,
            type_map: HashMap::new(),
            canonical_types: Vec::new(),
            canonical_type_indices: HashMap::new(),
            scoped_symbols: HashMap::new(),
            nominal_queue: Default::default(),
            raw_summaries,
        }
    }

    pub fn build(mut self) -> SemanticMetadata {
        // Hidden types may be ABI-reachable. Their binders must be scoped before
        // an exported signature or an ordered representation interns their types.
        for symbol in &self.provider.internal_symbols {
            if matches!(symbol.kind, luna_semantic::SymbolKind::Struct | luna_semantic::SymbolKind::Enum) {
                let owner = self.convert_symbol_id(&self.provider.symbol_canonicals[&symbol.id]);
                self.scope_parameters(&owner, &self.parameters(symbol.decl_id));
            }
        }
        // Assign binder identities before any type is interned in the public
        // registry. A binder belongs to a declaration and an ordinal, not T.
        for symbol in self.provider.exported_symbols.values() {
            self.scope_export_binders(symbol);
        }
        let mut owned_impls = Vec::new();
        for (&declaration, header) in &self.provider.checked_impl_headers {
            let trait_entry = self
                .provider
                .trait_impl_entries
                .iter()
                .find(|entry| entry.decl_id == Some(declaration));
            if trait_entry.is_none()
                && !self.provider.impl_method_symbols.iter().any(|method| {
                    method.sym.visibility != luna_ast::Visibility::Private
                        && method
                            .sym
                            .decl_id
                            .and_then(|d| self.provider.method_to_impl_decl.get(&d))
                            == Some(&declaration)
                })
            {
                continue;
            }
            let identity = StableSymbolId {
                provider_name: self.provider.name.clone(),
                symbol_path: format!(
                    "$impl/{}",
                    luna_llib::format::Fingerprint::from_slice(
                        self.impl_shape(declaration, header, trait_entry).as_bytes()
                    )
                ),
            };
            self.scope_parameters(&identity, &header.generic_params);
            for method in &self.provider.impl_method_symbols {
                if method
                    .sym
                    .decl_id
                    .and_then(|d| self.provider.method_to_impl_decl.get(&d))
                    == Some(&declaration)
                {
                    let canonical = &self.provider.symbol_canonicals[&method.sym.id];
                    let method_id = StableSymbolId {
                        provider_name: self.provider.name.clone(),
                        symbol_path: format!("{}::{}", identity.symbol_path, method.sym.name),
                    };
                    self.scoped_symbols
                        .insert(canonical.clone(), method_id.clone());
                    self.scope_parameters(&method_id, &self.parameters(method.sym.decl_id));
                }
            }
            owned_impls.push((identity, declaration, header, trait_entry));
        }
        owned_impls.sort_by(|a, b| a.0.cmp(&b.0));

        let mut exported_symbols = std::collections::BTreeMap::new();
        let mut sorted_exports: Vec<_> = self.provider.exported_symbols.iter().collect();
        sorted_exports.sort_by(|a, b| a.0.cmp(b.0));
        for (name, symbol) in sorted_exports {
            exported_symbols.insert(name.clone(), self.convert_exported_symbol(symbol));
        }
        let mut traits = std::collections::BTreeMap::new();
        Self::collect_trait_definitions(&exported_symbols, &mut traits);

        let mut impl_headers = Vec::new();
        for (identity, declaration, header, trait_entry) in owned_impls {
            let self_type = self.convert_type_id(header.self_type);
            let generic_params = header
                .generic_params
                .iter()
                .map(|param| self.convert_symbol_id(&self.provider.symbol_canonicals[param]))
                .collect();
            let constraints = self.convert_constraints(&header.generic_params);
            let mut methods = std::collections::BTreeMap::new();
            let mut method_contracts = std::collections::BTreeMap::new();
            let mut owned_methods: Vec<_> = self
                .provider
                .impl_method_symbols
                .iter()
                .filter(|method| {
                    method
                        .sym
                        .decl_id
                        .and_then(|d| self.provider.method_to_impl_decl.get(&d))
                        == Some(&declaration)
                        && (trait_entry.is_some()
                            || method.sym.visibility != luna_ast::Visibility::Private)
                })
                .collect();
            owned_methods.sort_by(|a, b| a.sym.name.cmp(&b.sym.name));
            for method in owned_methods {
                let contract = self.convert_exported_symbol(method);
                if let Some(ty) = contract.ty_index {
                    assert!(
                        methods.insert(method.sym.name.clone(), ty).is_none(),
                        "duplicate method in checked impl metadata"
                    );
                }
                method_contracts.insert(method.sym.name.clone(), contract);
            }
            let mut associated_types = std::collections::BTreeMap::new();
            let mut associated: Vec<_> = self
                .provider
                .decl_associated_types
                .iter()
                .filter(|((owner, _), _)| *owner == declaration)
                .collect();
            associated.sort_by_key(|((_, assoc), _)| self.convert_symbol_id(assoc));
            for ((_, associated), &ty) in associated {
                let identity = self.convert_symbol_id(associated);
                associated_types.insert(identity, self.convert_type_id(ty));
            }
            impl_headers.push(ImplHeader {
                identity,
                trait_id: trait_entry.map(|entry| self.convert_symbol_id(&entry.trait_id)),
                self_type,
                generic_params,
                trait_args: trait_entry
                    .map(|entry| {
                        entry
                            .trait_args
                            .iter()
                            .map(|&arg| self.convert_type_id(arg))
                            .collect()
                    })
                    .unwrap_or_default(),
                methods,
                constraints,
                method_contracts,
                associated_types,
            });
        }
        let nominal_layouts = self.build_nominal_layouts();
        SemanticMetadata {
            metadata_version: luna_llib::format::SEMANTIC_METADATA_VERSION,
            language_version: luna_llib::format::MLIB_COMPILER_VERSION,
            target_triple: "unknown".to_string(), // Set by writer.
            interface_fingerprint: luna_llib::format::Fingerprint([0; 32]),
            interface: CanonicalInterface {
                exported_symbols,
                types: self.canonical_types,
                traits,
                impl_headers,
                nominal_layouts,
            },
        }
    }

    fn build_nominal_layouts(&mut self) -> std::collections::BTreeMap<StableSymbolId, CanonicalNominalLayout> {
        use luna_semantic::SymbolKind;
        let mut layouts = std::collections::BTreeMap::new();
        while let Some((identity, symbol)) = self.nominal_queue.pop_first() {
            if layouts.contains_key(&identity) { continue; }
            let definition = self.provider.internal_symbols.iter().find(|s| s.id == symbol)
                .expect("owned nominal definition");
            let parameters = self.parameters(definition.decl_id);
            let ty = self.provider.types.get(self.provider.types.resolve(self.provider.symbol_types[&symbol])).clone();
            let members = match ty {
                SemanticType::Struct(_, _, fields) => {
                    let names = &self.provider.symbol_struct_field_names[&symbol];
                    assert_eq!(names.len(), fields.len(), "checked struct field order");
                    CanonicalNominalMembers::Struct(names.iter().zip(fields).map(|(name, ty)| {
                        let field = self.provider.internal_symbols.iter().find(|s|
                            Some(s.scope) == definition.inner_scope && s.name == *name);
                        CanonicalNominalField {
                            name: name.clone(), ty: self.convert_type_id(ty),
                            visibility: match field.map(|s| s.visibility) {
                                Some(luna_ast::Visibility::Private) => 0,
                                Some(luna_ast::Visibility::Internal) => 2,
                                _ => 1,
                            },
                        }
                    }).collect())
                }
                SemanticType::Enum(_, _, payloads) => {
                    let mut variants: Vec<_> = self.provider.internal_symbols.iter().filter_map(|s| {
                        if Some(s.scope) == definition.inner_scope {
                            if let SymbolKind::EnumVariant(index) = s.kind { return Some((index, s.name.clone())); }
                        }
                        None
                    }).collect();
                    variants.sort_by_key(|(index, _)| *index);
                    assert_eq!(variants.len(), payloads.len(), "checked enum variant order");
                    CanonicalNominalMembers::Enum(variants.into_iter().zip(payloads).enumerate().map(|(ordinal, ((index, name), ty))| {
                        assert_eq!(ordinal, index as usize, "checked enum discriminant");
                        CanonicalNominalVariant { name, payload: self.convert_type_id(ty) }
                    }).collect())
                }
                _ => unreachable!("nominal definition type"),
            };
            let layout = CanonicalNominalLayout {
                generic_params: parameters.iter().map(|p| self.convert_symbol_id(&self.provider.symbol_canonicals[p])).collect(),
                constraints: self.convert_constraints(&parameters), members,
                lifetime_contract: self.provider.symbol_type_lifetime_contracts.get(&symbol).cloned(),
                raw_storage_anchor_contract: self.provider.symbol_raw_storage_anchor_contracts.get(&symbol).cloned(),
            };
            layouts.insert(identity, layout);
        }
        layouts
    }

    fn queue_nominal(&mut self, symbol: luna_common::ids::SymbolId, identity: &StableSymbolId) {
        if identity.provider_name == self.provider.name {
            self.nominal_queue.insert(identity.clone(), symbol);
        }
    }

    fn parameters(&self, declaration: Option<luna_ast::DeclId>) -> Vec<luna_common::ids::SymbolId> {
        let Some(declaration) = declaration else {
            return Vec::new();
        };
        let mut parameters: Vec<_> = self
            .provider
            .raw_generic_param_symbols
            .iter()
            .filter(|((owner, _), _)| *owner == declaration)
            .map(|((_, position), &symbol)| (*position, symbol))
            .collect();
        parameters.sort_by_key(|(position, _)| *position);
        parameters.into_iter().map(|(_, symbol)| symbol).collect()
    }

    fn scope_parameters(
        &mut self,
        owner: &StableSymbolId,
        parameters: &[luna_common::ids::SymbolId],
    ) {
        for (index, parameter) in parameters.iter().enumerate() {
            self.scoped_symbols.insert(
                self.provider.symbol_canonicals[parameter].clone(),
                StableSymbolId {
                    provider_name: owner.provider_name.clone(),
                    symbol_path: format!("{}::$param{}", owner.symbol_path, index),
                },
            );
        }
    }

    fn scope_export_binders(&mut self, symbol: &ExternalSymbol) {
        if Self::owns_binders(symbol.sym.kind) {
            if let Some(canonical) = self.provider.symbol_canonicals.get(&symbol.sym.id) {
                let identity = self.convert_symbol_id(canonical);
                self.scope_parameters(&identity, &self.parameters(symbol.sym.decl_id));
            }
        }
        for child in symbol.children.values() {
            self.scope_export_binders(child);
        }
    }

    fn owns_binders(kind: luna_semantic::symbol::SymbolKind) -> bool {
        use luna_semantic::symbol::SymbolKind;
        matches!(
            kind,
            SymbolKind::Function
                | SymbolKind::ExternFunction
                | SymbolKind::TraitMethod
                | SymbolKind::Struct
                | SymbolKind::Enum
                | SymbolKind::Trait
                | SymbolKind::Alias
        )
    }

    fn collect_trait_definitions(
        symbols: &std::collections::BTreeMap<String, ExportedSymbol>,
        traits: &mut std::collections::BTreeMap<StableSymbolId, TraitDefinition>,
    ) {
        for symbol in symbols.values() {
            if symbol.kind == "Trait" {
                traits.insert(
                    symbol.symbol_id.clone(),
                    TraitDefinition {
                        name: symbol.symbol_id.symbol_path.clone(),
                        methods: symbol
                            .children
                            .iter()
                            .filter(|(_, child)| child.kind == "TraitMethod")
                            .filter_map(|(name, child)| child.ty_index.map(|ty| (name.clone(), ty)))
                            .collect(),
                        associated_types: symbol
                            .children
                            .values()
                            .filter(|child| child.kind == "AssociatedType")
                            .map(|child| child.symbol_id.clone())
                            .collect(),
                        generic_params: symbol.generic_params.clone(),
                        constraints: symbol.constraints.clone(),
                    },
                );
            }
            Self::collect_trait_definitions(&symbol.children, traits);
        }
    }

    // Structural sort keys carry only stable nominal paths and local binder
    // positions. They never include DeclId, SymbolId, source order or GP names.
    fn type_shape(&self, ty: SemanticTypeId, parameters: &[luna_common::ids::SymbolId]) -> String {
        let ty = self.provider.types.get(self.provider.types.resolve(ty));
        match ty {
            SemanticType::GenericParam(symbol) => parameters
                .iter()
                .position(|p| p == symbol)
                .map(|position| format!("param({position})"))
                .unwrap_or_else(|| {
                    format!(
                        "generic({:?})",
                        self.convert_symbol_id(&self.provider.symbol_canonicals[symbol])
                    )
                }),
            SemanticType::Struct(symbol, args, _) | SemanticType::Enum(symbol, args, _) => format!(
                "nominal({:?},{:?})",
                self.convert_symbol_id(&self.provider.symbol_canonicals[symbol]),
                args.iter()
                    .map(|&ty| self.type_shape(ty, parameters))
                    .collect::<Vec<_>>()
            ),
            SemanticType::Tuple(args) => format!(
                "tuple({:?})",
                args.iter()
                    .map(|&ty| self.type_shape(ty, parameters))
                    .collect::<Vec<_>>()
            ),
            SemanticType::Array(ty, len) => {
                format!("array({},{len})", self.type_shape(*ty, parameters))
            }
            SemanticType::Slice(ty) => format!("slice({})", self.type_shape(*ty, parameters)),
            SemanticType::Pointer(m, ty) => {
                format!("ptr({m:?},{})", self.type_shape(*ty, parameters))
            }
            SemanticType::Reference(_, m, ty) => {
                format!("ref({m:?},{})", self.type_shape(*ty, parameters))
            }
            SemanticType::Function { params, return_type, is_unsafe } => format!(
                "fn(unsafe={is_unsafe},{:?},{})",
                params
                    .iter()
                    .map(|&ty| self.type_shape(ty, parameters))
                    .collect::<Vec<_>>(),
                self.type_shape(*return_type, parameters)
            ),
            SemanticType::Projection {
                self_type,
                trait_id,
                assoc_type,
            } => format!(
                "projection({},{:?},{:?})",
                self.type_shape(*self_type, parameters),
                self.convert_symbol_id(&self.provider.symbol_canonicals[trait_id]),
                self.convert_symbol_id(&self.provider.symbol_canonicals[assoc_type])
            ),
            SemanticType::DynTrait(symbol) => format!(
                "dyn({:?})",
                self.convert_symbol_id(&self.provider.symbol_canonicals[symbol])
            ),
            SemanticType::Future(ty) => format!("future({})", self.type_shape(*ty, parameters)),
            SemanticType::Range(ty) => format!("range({})", self.type_shape(*ty, parameters)),
            SemanticType::Primitive(ty) => format!("primitive({ty:?})"),
            SemanticType::Void => "void".into(),
            SemanticType::Never => "never".into(),
            SemanticType::Error | SemanticType::InferenceVar(_) | SemanticType::Closure(..) => {
                panic!("unresolved/nonportable type in checked impl interface")
            }
        }
    }

    fn constraint_shapes(&self, parameters: &[luna_common::ids::SymbolId]) -> Vec<String> {
        let mut shapes = Vec::new();
        for (position, parameter) in parameters.iter().enumerate() {
            let canonical = &self.provider.symbol_canonicals[parameter];
            for bound in self
                .provider
                .trait_bounds
                .get(canonical)
                .into_iter()
                .flatten()
            {
                shapes.push(format!(
                    "trait({position},{:?},{:?})",
                    self.convert_symbol_id(&bound.trait_id),
                    bound
                        .trait_args
                        .iter()
                        .map(|&ty| self.type_shape(ty, parameters))
                        .collect::<Vec<_>>()
                ));
            }
            for bound in self
                .provider
                .assoc_type_bounds
                .get(canonical)
                .into_iter()
                .flatten()
            {
                shapes.push(format!(
                    "equality({position},{:?},{:?},{})",
                    self.convert_symbol_id(&bound.trait_id),
                    self.convert_symbol_id(&bound.assoc_sym),
                    self.type_shape(bound.target_ty, parameters)
                ));
            }
        }
        shapes.sort();
        shapes.dedup();
        shapes
    }

    fn impl_shape(
        &self,
        declaration: luna_ast::DeclId,
        header: &luna_semantic::semantic_tables::CheckedImplHeader,
        entry: Option<&crate::registry::ExternalTraitImplEntry>,
    ) -> String {
        let parameters = &header.generic_params;
        let mut associated: Vec<_> = self
            .provider
            .decl_associated_types
            .iter()
            .filter(|((owner, _), _)| *owner == declaration)
            .map(|((_, symbol), &ty)| {
                (
                    self.convert_symbol_id(symbol),
                    self.type_shape(ty, parameters),
                )
            })
            .collect();
        associated.sort();
        let mut methods = Vec::new();
        for method in &self.provider.impl_method_symbols {
            if method
                .sym
                .decl_id
                .and_then(|d| self.provider.method_to_impl_decl.get(&d))
                != Some(&declaration)
                || (entry.is_none() && method.sym.visibility == luna_ast::Visibility::Private)
            {
                continue;
            }
            let mut binders = parameters.clone();
            binders.extend(self.parameters(method.sym.decl_id));
            let signature = self.type_shape(self.provider.symbol_types[&method.sym.id], &binders);
            methods.push((
                method.sym.name.clone(),
                binders.len() - parameters.len(),
                signature,
                self.constraint_shapes(&binders),
            ));
        }
        methods.sort();
        format!(
            "impl({},{},{:?},{:?},{:?},{:?},{:?})",
            parameters.len(),
            self.type_shape(header.self_type, parameters),
            entry.map(|entry| self.convert_symbol_id(&entry.trait_id)),
            entry.map(|entry| entry
                .trait_args
                .iter()
                .map(|&ty| self.type_shape(ty, parameters))
                .collect::<Vec<_>>()),
            self.constraint_shapes(parameters),
            associated,
            methods
        )
    }

    fn convert_constraints(
        &mut self,
        parameters: &[luna_common::ids::SymbolId],
    ) -> GenericConstraints {
        let mut traits = Vec::new();
        let mut associated_equalities = Vec::new();
        for &parameter in parameters {
            let canonical = &self.provider.symbol_canonicals[&parameter];
            let param = self.convert_symbol_id(canonical);
            let mut bounds: Vec<_> = self
                .provider
                .trait_bounds
                .get(canonical)
                .into_iter()
                .flatten()
                .collect();
            bounds.sort_by_key(|bound| {
                (
                    self.convert_symbol_id(&bound.trait_id),
                    bound
                        .trait_args
                        .iter()
                        .map(|&ty| self.type_shape(ty, parameters))
                        .collect::<Vec<_>>(),
                )
            });
            for bound in bounds {
                let bound = CanonicalTraitBound {
                    param: param.clone(),
                    trait_id: self.convert_symbol_id(&bound.trait_id),
                    trait_args: bound
                        .trait_args
                        .iter()
                        .map(|&ty| self.convert_type_id(ty))
                        .collect(),
                };
                if !traits.contains(&bound) {
                    traits.push(bound);
                }
            }
            let mut equalities: Vec<_> = self
                .provider
                .assoc_type_bounds
                .get(canonical)
                .into_iter()
                .flatten()
                .collect();
            equalities.sort_by_key(|bound| {
                (
                    self.convert_symbol_id(&bound.trait_id),
                    self.convert_symbol_id(&bound.assoc_sym),
                    self.type_shape(bound.target_ty, parameters),
                )
            });
            for bound in equalities {
                let equality = CanonicalAssociatedEquality {
                    param: param.clone(),
                    trait_id: self.convert_symbol_id(&bound.trait_id),
                    associated_type: self.convert_symbol_id(&bound.assoc_sym),
                    target_type: self.convert_type_id(bound.target_ty),
                };
                if !associated_equalities.contains(&equality) {
                    associated_equalities.push(equality);
                }
            }
        }
        GenericConstraints {
            traits,
            associated_equalities,
        }
    }

    fn convert_symbol_id(&self, sym_id: &CanonicalSymbolId) -> StableSymbolId {
        if let Some(scoped) = self.scoped_symbols.get(sym_id) {
            return scoped.clone();
        }
        let provider_name = if sym_id.provider_id.0 == 0 || sym_id.provider_id == self.provider.id {
            self.provider.name.clone()
        } else {
            self.registry.get_provider_name(sym_id.provider_id)
        };
        StableSymbolId {
            provider_name,
            symbol_path: sym_id.name.clone(),
        }
    }

    fn convert_exported_symbol(&mut self, sym: &ExternalSymbol) -> ExportedSymbol {
        // Find its type if it exists: check sym.sym.id first, then merged_ids
        let mut ty_index = None;
        let sid = sym.sym.id;
        let symbol_ty = self.provider.symbol_types.get(&sid).copied().or_else(|| {
            sym.merged_ids
                .first()
                .and_then(|(_, msid)| self.provider.symbol_types.get(msid).copied())
        });
        if let Some(ty_id) = symbol_ty {
            ty_index = Some(self.convert_type_id(ty_id));
            if sym.sym.name == "Ok" || sym.sym.name == "Result" {}
        }
        if ty_index.is_none() && (sym.sym.name == "Ok" || sym.sym.name == "Result") {}

        let mut generic_params = Vec::new();
        let canon_opt = self.provider.symbol_canonicals.get(&sid).or_else(|| {
            sym.merged_ids
                .first()
                .and_then(|(_, msid)| self.provider.symbol_canonicals.get(msid))
        });
        if let Some(canon) = canon_opt {
            if let Some(gps) = self.provider.generic_param_symbols.get(canon) {
                generic_params = gps.iter().map(|p| self.convert_symbol_id(p)).collect();
            }
        }

        let kind_str = match sym.sym.kind {
            luna_semantic::symbol::SymbolKind::Function => "Function",
            luna_semantic::symbol::SymbolKind::ExternFunction => "ExternFunction",
            luna_semantic::symbol::SymbolKind::Variable => "Variable",
            luna_semantic::symbol::SymbolKind::Constant => "Constant",
            luna_semantic::symbol::SymbolKind::Struct => "Struct",
            luna_semantic::symbol::SymbolKind::Enum => "Enum",
            luna_semantic::symbol::SymbolKind::EnumVariant(_) => "EnumVariant",
            luna_semantic::symbol::SymbolKind::Trait => "Trait",
            luna_semantic::symbol::SymbolKind::TraitMethod => "TraitMethod",
            luna_semantic::symbol::SymbolKind::Alias => "Alias",
            luna_semantic::symbol::SymbolKind::AssociatedType => "AssociatedType",
            luna_semantic::symbol::SymbolKind::Module => "Module",
            luna_semantic::symbol::SymbolKind::Type => "Type",
            luna_semantic::symbol::SymbolKind::TypeParam => "TypeParam",
            luna_semantic::symbol::SymbolKind::LifetimeParam => "LifetimeParam",
            luna_semantic::symbol::SymbolKind::Macro => "Macro",
            luna_semantic::symbol::SymbolKind::Unknown => "Unknown",
        }
        .to_string();

        let mut children = std::collections::BTreeMap::new();
        let mut sorted_children: Vec<_> = sym.children.iter().collect();
        sorted_children.sort_by(|a, b| a.0.cmp(b.0));

        // Struct field symbols are not always entered in `symbol_types`.
        // Reconstruct each direct field's semantic type from the owner type
        // and the canonical field-name table, so serialized contracts can be
        // validated without making field ordinals portable identities.
        let field_types_by_name: HashMap<String, SemanticTypeId> = symbol_ty
            .and_then(
                |ty_id| match self.provider.types.get(self.provider.types.resolve(ty_id)) {
                    SemanticType::Struct(owner, _, field_types) => self
                        .provider
                        .symbol_struct_field_names
                        .get(owner)
                        .map(|names| {
                            names
                                .iter()
                                .cloned()
                                .zip(field_types.iter().copied())
                                .collect()
                        }),
                    _ => None,
                },
            )
            .unwrap_or_default();

        for (c_name, c_sym) in sorted_children {
            if matches!(
                sym.sym.kind,
                luna_semantic::symbol::SymbolKind::Function
                    | luna_semantic::symbol::SymbolKind::ExternFunction
                    | luna_semantic::symbol::SymbolKind::TraitMethod
            ) {
                continue;
            }
            if c_sym.sym.kind == luna_semantic::symbol::SymbolKind::TypeParam {
                continue;
            }
            let mut child = self.convert_exported_symbol(c_sym);
            if let Some(field_ty) = field_types_by_name.get(c_name) {
                child.ty_index = Some(self.convert_type_id(*field_ty));
            }
            children.insert(c_name.clone(), child);
        }

        let lifetime_contract = self
            .provider
            .symbol_lifetime_contracts
            .get(&sid)
            .or_else(|| {
                sym.merged_ids
                    .first()
                    .and_then(|(_, msid)| self.provider.symbol_lifetime_contracts.get(msid))
            })
            .cloned();

        let type_lifetime_contract = self
            .provider
            .symbol_type_lifetime_contracts
            .get(&sid)
            .or_else(|| {
                sym.merged_ids
                    .first()
                    .and_then(|(_, msid)| self.provider.symbol_type_lifetime_contracts.get(msid))
            })
            .cloned();

        let raw_storage_anchor_contract = self
            .provider
            .symbol_raw_storage_anchor_contracts
            .get(&sid)
            .or_else(|| {
                sym.merged_ids.first().and_then(|(_, msid)| {
                    self.provider.symbol_raw_storage_anchor_contracts.get(msid)
                })
            })
            .cloned();

        // Generic bodies are retained in AstInterface and reanalyzed by the
        // consumer. Export a body-derived raw-pointer summary for non-generic
        // functions whose ordinary concrete calls use the provider object.
        // The serialized form below contains parameter indices and canonical
        // field names only, never session-local compiler identities.
        let raw_pointer_effects = if matches!(kind_str.as_str(), "Function" | "ExternFunction")
            && generic_params.is_empty()
        {
            let candidate_ids = std::iter::once(sid)
                .chain(sym.merged_ids.iter().map(|(_, merged)| *merged))
                .collect::<std::collections::HashSet<_>>();
            let matching: Vec<_> = self
                .raw_summaries
                .iter()
                .filter(|(global, _)| {
                    global
                        .symbol_id
                        .is_some_and(|id| candidate_ids.contains(&id))
                })
                .map(|(_, summary)| summary)
                .collect();
            if matching.len() == 1 {
                let summary = matching[0];
                let direct_fields = summary
                    .raw_pointer_field_ret
                    .iter()
                    .map(|(name, field)| {
                        (
                            name.clone(),
                            canonical_raw_effect(&field.origin, &field.anchor),
                        )
                    })
                    .collect();
                Some(CanonicalRawPointerEffects {
                    call: crate::encode_call_effects(summary),
                    returned: canonical_raw_effect(
                        &summary.raw_pointer_ret,
                        &summary.raw_pointer_anchor_ret,
                    ),
                    direct_fields,
                })
            } else {
                None
            }
        } else {
            None
        };

        let is_unsafe = self.provider.unsafe_functions.contains(&sid)
            || sym
                .merged_ids
                .iter()
                .any(|(_, msid)| self.provider.unsafe_functions.contains(msid));

        let constraints = if Self::owns_binders(sym.sym.kind) {
            self.convert_constraints(&self.parameters(sym.sym.decl_id))
        } else {
            GenericConstraints::default()
        };
        let mut symbol_path = sym.sym.name.clone();
        if let Some(canon) = canon_opt {
            symbol_path = self.convert_symbol_id(canon).symbol_path;
        }

        let callable_signature = self.provider.callable_signatures.get(&sid).cloned()
            .or_else(|| sym.merged_ids.iter().find_map(|(_, symbol)| self.provider.callable_signatures.get(symbol).cloned()));
        ExportedSymbol {
            callable_signature,
            kind: kind_str,
            ty_index,
            visibility: match sym.sym.visibility {
                luna_ast::Visibility::Public => 1,
                luna_ast::Visibility::Internal => 2,
                luna_ast::Visibility::Private => 0,
            },
            generic_params,
            constraints,
            symbol_id: StableSymbolId {
                provider_name: self.provider.name.clone(),
                symbol_path,
            },
            children: children.into_iter().collect(),
            lifetime_contract,
            type_lifetime_contract,
            raw_storage_anchor_contract,
            raw_pointer_effects,
            is_unsafe,
        }
    }

    fn convert_type_id(&mut self, ty_id: SemanticTypeId) -> u32 {
        if let Some(&idx) = self.type_map.get(&ty_id) {
            return idx;
        }

        let sem_ty = self
            .provider
            .types
            .get(self.provider.types.resolve(ty_id))
            .clone();
        let canon = self.convert_semantic_type(&sem_ty);
        let idx = if let Some(&index) = self.canonical_type_indices.get(&canon) {
            index
        } else {
            let index = self.canonical_types.len() as u32;
            self.canonical_type_indices.insert(canon.clone(), index);
            self.canonical_types.push(canon);
            index
        };
        self.type_map.insert(ty_id, idx);
        idx
    }

    fn convert_semantic_type(&mut self, ty: &SemanticType) -> CanonicalType {
        match ty {
            SemanticType::Primitive(b) => CanonicalType::Primitive(*b),
            SemanticType::Struct(sym_id, targs, _bounds) => {
                let canon_sym = self.provider.symbol_canonicals.get(sym_id).unwrap();
                let stable_id = self.convert_symbol_id(canon_sym);
                self.queue_nominal(*sym_id, &stable_id);
                let args = targs.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Struct(stable_id, args, vec![])
            }
            SemanticType::Enum(sym_id, targs, _bounds) => {
                let canon_sym = self.provider.symbol_canonicals.get(sym_id).unwrap();
                let stable_id = self.convert_symbol_id(canon_sym);
                self.queue_nominal(*sym_id, &stable_id);
                let args = targs.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Enum(stable_id, args, vec![])
            }
            SemanticType::Tuple(tys) => {
                let args = tys.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Tuple(args)
            }
            SemanticType::Array(t, len) => CanonicalType::Array(self.convert_type_id(*t), *len),
            SemanticType::Slice(t) => CanonicalType::Slice(self.convert_type_id(*t)),
            SemanticType::Function { params, return_type, is_unsafe } => CanonicalType::Function {
                params: params.iter().map(|t| self.convert_type_id(*t)).collect(),
                return_type: self.convert_type_id(*return_type), is_unsafe: *is_unsafe },
            SemanticType::Pointer(mutability, t) => {
                CanonicalType::Pointer(mutability.clone(), self.convert_type_id(*t))
            }
            SemanticType::Reference(_lt, mutability, t) => CanonicalType::Reference(
                CanonicalLifetime::Anonymous,
                mutability.clone(),
                self.convert_type_id(*t),
            ),
            SemanticType::Void => CanonicalType::Void,
            SemanticType::Never => CanonicalType::Never,
            SemanticType::Error | SemanticType::InferenceVar(_) => CanonicalType::Error,
            SemanticType::GenericParam(sym_id) => {
                let canon_sym = self.provider.symbol_canonicals.get(sym_id).unwrap();
                CanonicalType::GenericParam(self.convert_symbol_id(canon_sym))
            }
            SemanticType::Closure(expr_id, captures, ret) => {
                let caps = captures.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Closure(expr_id.0 as u64, caps, self.convert_type_id(*ret))
            }
            SemanticType::DynTrait(sym_id) => {
                let canon_sym = self.provider.symbol_canonicals.get(sym_id).unwrap();
                CanonicalType::DynTrait(self.convert_symbol_id(canon_sym))
            }
            SemanticType::Future(t) => CanonicalType::Future(self.convert_type_id(*t)),
            SemanticType::Range(t) => CanonicalType::Range(self.convert_type_id(*t)),
            SemanticType::Projection {
                self_type,
                trait_id,
                assoc_type,
            } => {
                let canon_trait = self.provider.symbol_canonicals.get(trait_id).unwrap();
                let canon_assoc = self.provider.symbol_canonicals.get(assoc_type).unwrap();
                CanonicalType::Projection {
                    self_type: self.convert_type_id(*self_type),
                    trait_id: self.convert_symbol_id(canon_trait),
                    assoc_type: self.convert_symbol_id(canon_assoc),
                }
            }
        }
    }
}

fn canonical_raw_effect(
    origin: &RawPointerReturnEffect,
    anchor: &RawPointerAnchorReturnEffect,
) -> CanonicalRawPointerEffect {
    let origin = match origin {
        RawPointerReturnEffect::Independent => CanonicalRawPointerOrigin::Independent,
        RawPointerReturnEffect::From(params) => {
            let converted = params
                .iter()
                .map(|index| u32::try_from(*index))
                .collect::<Result<Vec<_>, _>>();
            converted
                .map(CanonicalRawPointerOrigin::FromParameters)
                .unwrap_or(CanonicalRawPointerOrigin::Unknown)
        }
        RawPointerReturnEffect::Unknown => CanonicalRawPointerOrigin::Unknown,
    };
    let anchor = match anchor {
        RawPointerAnchorReturnEffect::Independent => CanonicalRawPointerAnchor::Independent,
        RawPointerAnchorReturnEffect::Unknown => CanonicalRawPointerAnchor::Unknown,
        RawPointerAnchorReturnEffect::From(sources) => {
            let converted = sources
                .iter()
                .map(|source| match source {
                    RawPointerAnchorSource::RawParam(index) => u32::try_from(*index)
                        .map(CanonicalRawPointerAnchorSource::RawParameter)
                        .map_err(|_| ()),
                    RawPointerAnchorSource::OwnerField { param, field } => u32::try_from(*param)
                        .map(|parameter| CanonicalRawPointerAnchorSource::OwnerField {
                            parameter,
                            field_name: field.clone(),
                        })
                        .map_err(|_| ()),
                    RawPointerAnchorSource::Unknown => Ok(CanonicalRawPointerAnchorSource::Unknown),
                })
                .collect::<Result<Vec<_>, _>>();
            converted
                .map(CanonicalRawPointerAnchor::From)
                .unwrap_or(CanonicalRawPointerAnchor::Unknown)
        }
    };
    CanonicalRawPointerEffect { origin, anchor }
}
