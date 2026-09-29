use crate::registry::{CanonicalSymbolId, ExternalImplKey, ExternalSymbol, ExternalTraitImplEntry, ModuleRegistry, ProviderInterface};
use luna_llib::metadata::{
    CanonicalInterface, CanonicalLifetime, CanonicalRawPointerAnchor, CanonicalRawPointerAnchorSource,
    CanonicalRawPointerEffect, CanonicalRawPointerEffects, CanonicalRawPointerOrigin,
    CanonicalType, ExportedSymbol, ImplHeader,
    SemanticMetadata, StableSymbolId, TraitDefinition,
};
use luna_semantic::ty::{SemanticType, SemanticTypeId, TypeContext};
use luna_borrowck::effect::{CallEffectSummary, RawPointerAnchorReturnEffect, RawPointerAnchorSource, RawPointerReturnEffect};
use luna_mvir::GlobalId;
use std::collections::HashMap;

pub struct MetadataBuilder<'a> {
    registry: &'a ModuleRegistry,
    provider: &'a ProviderInterface,
    type_map: HashMap<SemanticTypeId, u32>,
    canonical_types: Vec<CanonicalType>,
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
            raw_summaries,
        }
    }

    pub fn build(mut self) -> SemanticMetadata {
        // Pre-allocate to prevent infinite recursion on self-referential types
        // Actually we should traverse and build

        let mut exported_symbols = std::collections::BTreeMap::new();
        let mut sorted_exports: Vec<_> = self.provider.exported_symbols.iter().collect();
        sorted_exports.sort_by(|a, b| a.0.cmp(b.0));
        
        for (name, ext_sym) in sorted_exports {
            let exported = self.convert_exported_symbol(ext_sym);
            exported_symbols.insert(name.clone(), exported);
        }

        let mut traits = std::collections::BTreeMap::new();
        // The trait implementations and definitions
        // Wait, how do we get TraitDefinition?
        // luna_semantic hasn't exposed trait fully, but we have trait_impl_entries
        // We will build what we can

        let mut impl_headers = Vec::new();
        let mut processed_keys = std::collections::HashSet::new();

        // Deterministic sorting for impls
        #[derive(PartialEq, Eq, PartialOrd, Ord)]
        enum SelfTypeSort {
            Primitive(u8),
            Nominal(String, String),
        }
        #[derive(PartialEq, Eq, PartialOrd, Ord)]
        struct ImplSortKey {
            trait_path: Option<(String, String)>,
            self_type: SelfTypeSort,
        }

        let mut sorted_impls: Vec<_> = self.provider.impl_methods.iter().collect();
        sorted_impls.sort_by_key(|(k, _)| {
            let trait_path = k.trait_id.as_ref().map(|t| {
                let stable = self.convert_symbol_id(t);
                (stable.provider_name, stable.symbol_path)
            });
            let self_type = match &k.self_type_def {
                crate::registry::ExternalImplSelfTypeKey::Primitive(b) => SelfTypeSort::Primitive(*b as u8),
                crate::registry::ExternalImplSelfTypeKey::Nominal(n) => {
                    let stable = self.convert_symbol_id(n);
                    SelfTypeSort::Nominal(stable.provider_name, stable.symbol_path)
                }
            };
            ImplSortKey { trait_path, self_type }
        });

        for (impl_key, method_canons) in sorted_impls {
            processed_keys.insert(impl_key.clone());
            let trait_id = impl_key.trait_id.as_ref().map(|t| self.convert_symbol_id(t));
            let self_type = if let Some(&st) = self.provider.impl_self_types.get(impl_key) {
                self.convert_type_id(st)
            } else {
                match &impl_key.self_type_def {
                    crate::registry::ExternalImplSelfTypeKey::Nominal(canon_def) => {
                        let stable = self.convert_symbol_id(canon_def);
                        let ty = self.canonical_types.len() as u32;
                        self.canonical_types.push(CanonicalType::Struct(stable, vec![], vec![]));
                        ty
                    }
                    crate::registry::ExternalImplSelfTypeKey::Primitive(b) => {
                        let ty = self.canonical_types.len() as u32;
                        self.canonical_types.push(CanonicalType::Primitive(*b));
                        ty
                    }
                }
            };

            let generic_params = self.provider.impl_generic_params.get(impl_key)
                .map(|gps| gps.iter().map(|p| self.convert_symbol_id(p)).collect())
                .unwrap_or_default();

                let mut methods = std::collections::BTreeMap::new();
                for m_canon in method_canons {
                    let m_ty_opt = self.provider.impl_method_symbols.iter()
                        .find(|s| s.sym.name == m_canon.name && (m_canon.decl_id.is_none() || s.sym.decl_id == m_canon.decl_id))
                        .and_then(|s| self.provider.symbol_types.get(&s.sym.id).copied());
                    if let Some(m_ty) = m_ty_opt {
                        let ty_idx = self.convert_type_id(m_ty);
                        methods.insert(m_canon.name.clone(), ty_idx);
                    }
                }

                let actual_self_type_id_opt = self.provider.impl_self_types.get(impl_key).copied();

                let mut found_trait_args = Vec::new();
                for e in &self.provider.trait_impl_entries {
                    if Some(e.trait_id.clone()) == impl_key.trait_id {
                        if let Some(actual_st) = actual_self_type_id_opt {
                            if e.self_type == actual_st {
                                found_trait_args = e.trait_args.iter().map(|&a| self.convert_type_id(a)).collect();
                                break;
                            }
                        } else {
                            let self_type_def = match self.provider.types.get(e.self_type) {
                                SemanticType::Primitive(b) => crate::registry::ExternalImplSelfTypeKey::Primitive(*b),
                                _ => crate::registry::ExternalImplSelfTypeKey::Nominal(
                                    self.get_canonical_from_type(e.self_type).unwrap_or_else(|| e.trait_id.clone())
                                ),
                            };
                            if self_type_def == impl_key.self_type_def {
                                found_trait_args = e.trait_args.iter().map(|&a| self.convert_type_id(a)).collect();
                                break;
                            }
                        }
                    }
                }

                impl_headers.push(ImplHeader {
                    trait_id,
                    self_type,
                    generic_params,
                    trait_args: found_trait_args,
                    methods,
                });
            }

            let mut sorted_trait_entries: Vec<_> = self.provider.trait_impl_entries.iter().collect();
            sorted_trait_entries.sort_by_key(|e| {
                let stable = self.convert_symbol_id(&e.trait_id);
                (stable.provider_name, stable.symbol_path)
            });

            for entry in sorted_trait_entries {
                let self_type_def = match self.provider.types.get(entry.self_type) {
                    SemanticType::Primitive(b) => crate::registry::ExternalImplSelfTypeKey::Primitive(*b),
                    _ => crate::registry::ExternalImplSelfTypeKey::Nominal(
                        self.get_canonical_from_type(entry.self_type).unwrap_or_else(|| entry.trait_id.clone())
                    ),
                };
                let key = ExternalImplKey {
                    trait_id: Some(entry.trait_id.clone()),
                    self_type_def,
                };
                if !processed_keys.contains(&key) {
                    let trait_id = Some(self.convert_symbol_id(&entry.trait_id));
                    let self_type = self.convert_type_id(entry.self_type);
                    let generic_params = entry.generic_params.iter().map(|p| self.convert_symbol_id(p)).collect();
                    let trait_args = entry.trait_args.iter().map(|&a| self.convert_type_id(a)).collect();
                    impl_headers.push(ImplHeader {
                        trait_id,
                        self_type,
                        generic_params,
                        trait_args,
                        methods: std::collections::BTreeMap::new(),
                    });
                }
            }

        let interface = CanonicalInterface {
            exported_symbols: exported_symbols.into_iter().collect(),
            types: self.canonical_types.clone(),
            traits: traits.into_iter().collect(),
            impl_headers,
        };

        SemanticMetadata {
            metadata_version: luna_llib::format::SEMANTIC_METADATA_VERSION,
            language_version: luna_llib::format::MLIB_COMPILER_VERSION,
            target_triple: "unknown".to_string(), // Set by writer later
            interface_fingerprint: luna_llib::format::Fingerprint([0; 32]), // Set by writer later
            interface,
        }
    }

    fn get_canonical_from_type(&self, ty_id: SemanticTypeId) -> Option<CanonicalSymbolId> {
        let ty = self.provider.types.get(ty_id);
        match ty {
            SemanticType::Struct(sym_id, _, _) | SemanticType::Enum(sym_id, _, _) => {
                self.provider.symbol_canonicals.get(sym_id).cloned()
            }
            _ => None,
        }
    }

    fn convert_symbol_id(&self, sym_id: &CanonicalSymbolId) -> StableSymbolId {
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
        let symbol_ty = self.provider.symbol_types.get(&sid)
            .copied()
            .or_else(|| sym.merged_ids.first().and_then(|(_, msid)| self.provider.symbol_types.get(msid).copied()));
        if let Some(ty_id) = symbol_ty {
            ty_index = Some(self.convert_type_id(ty_id));
            if sym.sym.name == "Ok" || sym.sym.name == "Result" {
            }
        }
        if ty_index.is_none() && (sym.sym.name == "Ok" || sym.sym.name == "Result") {
        }
        
        let mut generic_params = Vec::new();
        let canon_opt = self.provider.symbol_canonicals.get(&sid)
            .or_else(|| sym.merged_ids.first().and_then(|(_, msid)| self.provider.symbol_canonicals.get(msid)));
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
        }.to_string();

        let mut children = std::collections::BTreeMap::new();
        let mut sorted_children: Vec<_> = sym.children.iter().collect();
        sorted_children.sort_by(|a, b| a.0.cmp(b.0));

        // Struct field symbols are not always entered in `symbol_types`.
        // Reconstruct each direct field's semantic type from the owner type
        // and the canonical field-name table, so serialized contracts can be
        // validated without making field ordinals portable identities.
        let field_types_by_name: HashMap<String, SemanticTypeId> = symbol_ty
            .and_then(|ty_id| match self.provider.types.get(self.provider.types.resolve(ty_id)) {
                SemanticType::Struct(owner, _, field_types) => {
                    self.provider.symbol_struct_field_names.get(owner).map(|names| {
                        names.iter().cloned().zip(field_types.iter().copied()).collect()
                    })
                }
                _ => None,
            })
            .unwrap_or_default();
        
        for (c_name, c_sym) in sorted_children {
            let mut child = self.convert_exported_symbol(c_sym);
            if let Some(field_ty) = field_types_by_name.get(c_name) {
                child.ty_index = Some(self.convert_type_id(*field_ty));
            }
            children.insert(c_name.clone(), child);
        }

        let lifetime_contract = self.provider.symbol_lifetime_contracts.get(&sid)
            .or_else(|| sym.merged_ids.first().and_then(|(_, msid)| self.provider.symbol_lifetime_contracts.get(msid)))
            .cloned();

        let type_lifetime_contract = self.provider.symbol_type_lifetime_contracts.get(&sid)
            .or_else(|| sym.merged_ids.first().and_then(|(_, msid)| self.provider.symbol_type_lifetime_contracts.get(msid)))
            .cloned();

        let raw_storage_anchor_contract = self.provider.symbol_raw_storage_anchor_contracts.get(&sid)
            .or_else(|| sym.merged_ids.first().and_then(|(_, msid)| self.provider.symbol_raw_storage_anchor_contracts.get(msid)))
            .cloned();

        // Generic bodies are retained in AstInterface and reanalyzed by the
        // consumer. Export a body-derived raw-pointer summary only for
        // non-generic functions whose body is not present in the artifact.
        // The serialized form below contains parameter indices and canonical
        // field names only, never session-local compiler identities.
        let raw_pointer_effects = if matches!(kind_str.as_str(), "Function" | "ExternFunction")
            && generic_params.is_empty()
        {
            let candidate_ids = std::iter::once(sid)
                .chain(sym.merged_ids.iter().map(|(_, merged)| *merged))
                .collect::<std::collections::HashSet<_>>();
            let matching: Vec<_> = self.raw_summaries.iter()
                .filter(|(global, _)| global.symbol_id.is_some_and(|id| candidate_ids.contains(&id)))
                .map(|(_, summary)| summary)
                .collect();
            if matching.len() == 1 {
                let summary = matching[0];
                let direct_fields = summary.raw_pointer_field_ret.iter()
                    .map(|(name, field)| (name.clone(), canonical_raw_effect(&field.origin, &field.anchor)))
                    .collect();
                Some(CanonicalRawPointerEffects {
                    returned: canonical_raw_effect(&summary.raw_pointer_ret, &summary.raw_pointer_anchor_ret),
                    direct_fields,
                })
            } else {
                None
            }
        } else {
            None
        };


        let is_unsafe = self.provider.unsafe_functions.contains(&sid)
            || sym.merged_ids.iter().any(|(_, msid)| self.provider.unsafe_functions.contains(msid));

        let mut symbol_path = sym.sym.name.clone();
        if let Some(canon) = canon_opt {
            symbol_path = canon.name.clone();
        }

        ExportedSymbol {
            kind: kind_str,
            ty_index,
            visibility: match sym.sym.visibility {
                luna_ast::Visibility::Public => 1,
                luna_ast::Visibility::Internal => 2,
                luna_ast::Visibility::Private => 0,
            },
            generic_params,
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

        let idx = self.canonical_types.len() as u32;
        self.type_map.insert(ty_id, idx);
        
        // Insert a dummy to allocate slot
        self.canonical_types.push(CanonicalType::Error);
        
        let sem_ty = self.provider.types.get(ty_id).clone();
        let canon = self.convert_semantic_type(&sem_ty);
        
        self.canonical_types[idx as usize] = canon;
        idx
    }

    fn convert_semantic_type(&mut self, ty: &SemanticType) -> CanonicalType {
        match ty {
            SemanticType::Primitive(b) => CanonicalType::Primitive(*b),
            SemanticType::Struct(sym_id, targs, _bounds) => {
                let canon_sym = self.provider.symbol_canonicals.get(sym_id).unwrap();
                let stable_id = self.convert_symbol_id(canon_sym);
                let args = targs.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Struct(stable_id, args, vec![])
            }
            SemanticType::Enum(sym_id, targs, _bounds) => {
                let canon_sym = self.provider.symbol_canonicals.get(sym_id).unwrap();
                let stable_id = self.convert_symbol_id(canon_sym);
                let args = targs.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Enum(stable_id, args, vec![])
            }
            SemanticType::Tuple(tys) => {
                let args = tys.iter().map(|t| self.convert_type_id(*t)).collect();
                CanonicalType::Tuple(args)
            }
            SemanticType::Array(t, len) => CanonicalType::Array(self.convert_type_id(*t), *len),
            SemanticType::Slice(t) => CanonicalType::Slice(self.convert_type_id(*t)),
            SemanticType::Function { params, return_type } => CanonicalType::Function {
                params: params.iter().map(|t| self.convert_type_id(*t)).collect(),
                return_type: self.convert_type_id(*return_type),
            },
            SemanticType::Pointer(mutability, t) => CanonicalType::Pointer(mutability.clone(), self.convert_type_id(*t)),
            SemanticType::Reference(_lt, mutability, t) => CanonicalType::Reference(CanonicalLifetime::Anonymous, mutability.clone(), self.convert_type_id(*t)),
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
            SemanticType::Projection { self_type, trait_id, assoc_type } => {
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
            let converted = params.iter().map(|index| u32::try_from(*index)).collect::<Result<Vec<_>, _>>();
            converted.map(CanonicalRawPointerOrigin::FromParameters)
                .unwrap_or(CanonicalRawPointerOrigin::Unknown)
        }
        RawPointerReturnEffect::Unknown => CanonicalRawPointerOrigin::Unknown,
    };
    let anchor = match anchor {
        RawPointerAnchorReturnEffect::Independent => CanonicalRawPointerAnchor::Independent,
        RawPointerAnchorReturnEffect::Unknown => CanonicalRawPointerAnchor::Unknown,
        RawPointerAnchorReturnEffect::From(sources) => {
            let converted = sources.iter().map(|source| match source {
                RawPointerAnchorSource::RawParam(index) => u32::try_from(*index)
                    .map(CanonicalRawPointerAnchorSource::RawParameter).map_err(|_| ()),
                RawPointerAnchorSource::OwnerField { param, field } => u32::try_from(*param)
                    .map(|parameter| CanonicalRawPointerAnchorSource::OwnerField {
                        parameter,
                        field_name: field.clone(),
                    }).map_err(|_| ()),
                RawPointerAnchorSource::Unknown => Ok(CanonicalRawPointerAnchorSource::Unknown),
            }).collect::<Result<Vec<_>, _>>();
            converted.map(CanonicalRawPointerAnchor::From)
                .unwrap_or(CanonicalRawPointerAnchor::Unknown)
        }
    };
    CanonicalRawPointerEffect { origin, anchor }
}
