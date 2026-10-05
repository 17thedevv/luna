use luna_ast::{ExprId, StmtId, DeclId, PatId, TypeId as AstTypeId};
use luna_common::ids::SymbolId;
use std::collections::{HashMap, HashSet};

use crate::ty::SemanticTypeId;
use crate::ScopeId;
use crate::symbol::SymbolTable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureMode {
    SharedBorrow,
    MutableBorrow,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaptureBinding {
    pub symbol: SymbolId,
    pub mode: CaptureMode,
    pub env_field: u32,
    pub ty: SemanticTypeId,
    pub env_ty: SemanticTypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplSelfTypeKey {
    Nominal(SymbolId),
    Primitive(crate::ty::BuiltinType),
    Slice,
}

impl From<SymbolId> for ImplSelfTypeKey {
    fn from(sym: SymbolId) -> Self {
        ImplSelfTypeKey::Nominal(sym)
    }
}

impl From<crate::ty::BuiltinType> for ImplSelfTypeKey {
    fn from(b: crate::ty::BuiltinType) -> Self {
        ImplSelfTypeKey::Primitive(b)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplKey {
    pub trait_id: Option<SymbolId>,
    pub self_type_def: ImplSelfTypeKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitBound {
    pub param: SymbolId,
    pub trait_id: SymbolId,
    pub trait_args: Vec<SemanticTypeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitImplEntry {
    pub decl_id: Option<DeclId>,
    pub trait_id: SymbolId,
    pub self_type: SemanticTypeId,
    pub generic_params: Vec<SymbolId>,
    pub trait_args: Vec<SemanticTypeId>,
}

/// A checked impl binder and self pattern, owned by one declaration. ImplKey
/// groups candidates by nominal head; it cannot identify a particular impl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedImplHeader {
    pub self_type: SemanticTypeId,
    pub generic_params: Vec<SymbolId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitResolution {
    pub trait_id: SymbolId,
    pub method_sym: SymbolId,
}

/// Fully resolved language-protocol plan for a `for pattern in iterable` loop.
///
/// This records semantic identities selected by the type checker so lowering
/// never needs to recognize a concrete container or rediscover an impl by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForLoopResolution {
    pub into_iter_method: SymbolId,
    pub into_iter_subst: crate::ty::Substitution,
    pub next_method: SymbolId,
    pub next_subst: crate::ty::Substitution,
    pub iterator_type: SemanticTypeId,
    pub item_type: SemanticTypeId,
    pub option_type: SemanticTypeId,
    pub next_receiver_type: SemanticTypeId,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntrinsicKind {
    Null,
    Cast,
    PtrOffset,
    PtrWrite,
    SizeOf,
    AlignOf,
    TypeOf,
    TypeInfo,
}

/// Declaration names belong to callable contracts, never structural function types.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CallableSignature {
    pub parameter_names: Vec<String>,
    /// Canonical token contracts, without source offsets or session identities.
    /// Executable definitions live in the portable AST, not in this interface.
    pub default_contracts: Vec<Option<String>>,
    pub has_receiver: bool,
    pub is_variadic: bool,
}

/// Source expressions remain in evaluation order. Ordinals describe ABI binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallArgumentBinding {
    pub source_to_parameter: Vec<u32>,
}
impl CallArgumentBinding {
    pub fn in_parameter_order<T: Clone>(&self, source_values: &[T]) -> Option<Vec<T>> {
        if source_values.len() != self.source_to_parameter.len() { return None; }
        let mut ordered = vec![None; source_values.len()];
        for (value, &ordinal) in source_values.iter().zip(&self.source_to_parameter) {
            let slot = ordered.get_mut(ordinal as usize)?;
            if slot.is_some() { return None; }
            *slot = Some(value.clone());
        }
        ordered.into_iter().collect()
    }
}

#[cfg(test)]
mod call_binding_tests {
    use super::CallArgumentBinding;

    #[test]
    fn checked_binding_reorders_values_and_rejects_non_bijective_plans() {
        let plan = CallArgumentBinding { source_to_parameter: vec![1, 0] };
        assert_eq!(plan.in_parameter_order(&[2, 9]), Some(vec![9, 2]));
        for ordinals in [vec![0, 0], vec![0, 2], vec![0], vec![0, 1, 2]] {
            assert!(CallArgumentBinding { source_to_parameter: ordinals }.in_parameter_order(&[2, 9]).is_none());
        }
    }
}

#[derive(Clone)]
pub struct SemanticTables {
    pub callable_signatures: HashMap<SymbolId, CallableSignature>,
    pub call_argument_bindings: HashMap<ExprId, CallArgumentBinding>,
    pub expr_types: HashMap<ExprId, SemanticTypeId>,
    pub expr_symbols: HashMap<ExprId, SymbolId>,
    pub expr_substs: HashMap<ExprId, crate::ty::Substitution>,
    pub expr_trait_resolutions: HashMap<ExprId, TraitResolution>,
    pub try_branch_methods: HashMap<ExprId, luna_ast::DeclId>,
    pub try_branch_substs: HashMap<ExprId, crate::ty::Substitution>,
    pub try_branch_return_types: HashMap<ExprId, SemanticTypeId>,
    pub try_from_residual_methods: HashMap<ExprId, luna_ast::DeclId>,
    pub try_from_residual_substs: HashMap<ExprId, crate::ty::Substitution>,
    pub intrinsic_types: HashMap<ExprId, SemanticTypeId>,
    pub expr_intrinsics: HashMap<ExprId, IntrinsicKind>,
    pub expr_member_indices: HashMap<ExprId, u32>,
    pub expr_struct_init_indices: HashMap<ExprId, Vec<u32>>,
    

    pub expr_sizeof_target: HashMap<ExprId, SemanticTypeId>,
    pub expr_lifetimes: HashMap<ExprId, crate::ty::LifetimeId>,
    pub expr_captures: HashMap<ExprId, Vec<SymbolId>>,
    pub closure_capture_bindings: HashMap<ExprId, Vec<CaptureBinding>>,
    pub closure_mutated_captures: HashMap<ExprId, HashSet<SymbolId>>,
    pub closure_env_types: HashMap<ExprId, SemanticTypeId>,
    pub closure_env_ptr_types: HashMap<ExprId, SemanticTypeId>,
    
    // Dynamic dispatch and coercion tables
    pub dyn_method_indices: HashMap<ExprId, u32>,
    pub coercions: HashMap<ExprId, crate::coercion::CoercionKind>,
    
    // Try operator branches: (inner_success_idx, inner_failure_idx, func_failure_idx)
    pub try_branches: HashMap<ExprId, (u32, u32, u32)>,
    
    // For loop desugaring tracking
    pub for_loop_next: HashMap<StmtId, SymbolId>,
    pub for_loop_subst: HashMap<StmtId, crate::ty::Substitution>,
    pub for_loop_resolutions: HashMap<StmtId, ForLoopResolution>,
    
    pub pat_symbols: HashMap<PatId, SymbolId>,
    pub pat_types: HashMap<PatId, SemanticTypeId>,
    
    pub decl_symbols: HashMap<DeclId, SymbolId>,
    pub symbol_decls: HashMap<SymbolId, DeclId>,
    
    pub type_symbols: HashMap<AstTypeId, SymbolId>,
    
    pub ast_type_to_semantic: HashMap<AstTypeId, SemanticTypeId>,
    pub symbol_types: HashMap<SymbolId, SemanticTypeId>,
    pub type_scopes: HashMap<AstTypeId, ScopeId>,
    pub decl_scopes: HashMap<DeclId, ScopeId>,
    
    // Maps a function's SymbolId to a boolean vector indicating which parameters are @sync_noescape
    pub ffi_sync_noescape: HashMap<SymbolId, Vec<bool>>,
    /// Function symbols declared inside an `extern` item. Kept distinct from
    /// `SymbolKind` because safe-reference FFI calls have their own safe,
    /// synchronous contract while raw-pointer FFI accesses are call-scoped.
    pub extern_functions: HashSet<SymbolId>,
    /// Imported function definitions already supplied by a provider object.
    /// Concrete calls use that object; generic/comptime bodies remain portable.
    pub object_backed_functions: HashSet<SymbolId>,
    /// Evaluation may materialize dependency bodies without a runtime call.
    /// Until precise evaluation dependencies are returned by the VM, retain
    /// execution identities conservatively for that compilation.
    pub evaluated_comptime: bool,
    
    // Maps a (DeclId, param_index) to the SymbolId of the generic parameter
    pub generic_param_symbols: HashMap<(DeclId, usize), SymbolId>,
    
    // Structs that implement Drop -> the drop function's SymbolId
    pub drop_impls: HashMap<SymbolId, SymbolId>,
    
    // Canonical impl resolution: ImplKey -> Impl DeclId
    pub trait_impls: HashMap<ImplKey, Vec<DeclId>>,
    
    // Maps a Trait's SymbolId to its required method SymbolIds
    pub trait_methods: HashMap<SymbolId, Vec<SymbolId>>,
    
    // Maps a Struct's SymbolId to its field SymbolIds
    pub struct_fields: HashMap<SymbolId, Vec<SymbolId>>,
    /// Canonical declaration-order field names. Unlike `struct_fields`, this
    /// remains usable for imported types whose field SymbolIds are relocated.
    pub struct_field_names: HashMap<SymbolId, Vec<String>>,
    
    // Maps a GenericParam's SymbolId to its TraitBounds
    pub trait_bounds: HashMap<SymbolId, Vec<TraitBound>>,
    
    // Maps a Trait's SymbolId to its generic parameter SymbolIds
    pub trait_generic_params: HashMap<SymbolId, Vec<SymbolId>>,
    
    // Maps a base struct/enum SymbolId to a list of its method SymbolIds (from inherent impl blocks without trait)
    pub impl_methods: HashMap<ImplKey, Vec<SymbolId>>,
    
    // Maps a method SymbolId to its parent impl block DeclId
    pub method_impls: HashMap<SymbolId, ImplKey>,
    pub method_to_impl_decl: HashMap<DeclId, DeclId>,
    pub method_sym_to_impl_decl: HashMap<SymbolId, DeclId>,
    
    // Maps a Trait's SymbolId to its declared associated type SymbolIds
    pub trait_associated_types: HashMap<SymbolId, Vec<SymbolId>>,
    // Maps an associated type SymbolId to its declaring Trait's SymbolId
    pub assoc_type_traits: HashMap<SymbolId, SymbolId>,
    // Maps (ImplKey, assoc_type_sym) to the declared aliased SemanticTypeId in that impl
    pub impl_associated_types: HashMap<(ImplKey, SymbolId), SemanticTypeId>,
    // Maps (trait_sym, name) to assoc_type_sym
    pub assoc_type_names: HashMap<(SymbolId, String), SymbolId>,
    // Maps a generic param symbol to its associated type equality bounds: (trait_id, assoc_type_sym, target_ty)
    pub assoc_type_bounds: HashMap<SymbolId, Vec<(SymbolId, SymbolId, SemanticTypeId)>>,
    // Maps ImplKey to lowered SemanticTypeId of self_type in that impl (e.g., Result<T, E>)
    pub impl_self_types: HashMap<ImplKey, SemanticTypeId>,
    pub checked_impl_headers: HashMap<DeclId, CheckedImplHeader>,
    // Maps ImplKey to list of generic parameter symbols declared on the impl block
    pub impl_generic_params: HashMap<ImplKey, Vec<SymbolId>>,
    pub trait_impl_entries: Vec<TraitImplEntry>,
    pub decl_associated_types: HashMap<(DeclId, SymbolId), SemanticTypeId>,

    pub macro_decls: HashMap<SymbolId, DeclId>,
    pub decl_macros: HashMap<DeclId, SymbolId>,
    pub function_effects: HashMap<SymbolId, crate::effect::EffectSet>,
    pub unsafe_functions: HashSet<SymbolId>,
    pub fn_lifetime_contracts: HashMap<SymbolId, crate::CanonicalLifetimeContract>,
    pub type_lifetime_contracts: HashMap<SymbolId, crate::CanonicalTypeLifetimeContract>,
    pub resolved_type_lifetime_contracts: HashMap<SymbolId, crate::ResolvedTypeLifetimeContract>,
    pub raw_storage_anchor_contracts: HashMap<SymbolId, crate::CanonicalRawStorageAnchorContract>,
    pub resolved_raw_storage_anchor_contracts: HashMap<SymbolId, crate::ResolvedRawStorageAnchorContract>,
}

impl SemanticTables {
    /// Resolve a named method for a concrete receiver deterministically.
    ///
    /// Inherent methods take precedence over trait methods.  This is also used
    /// by later pipeline stages when imported metadata does not carry an
    /// expression-local method symbol, so it must not depend on HashMap order.
    pub fn find_method_prefer_inherent(
        &self,
        self_type_def: ImplSelfTypeKey,
        name: &str,
        symbols: &SymbolTable,
    ) -> Option<(SymbolId, ImplKey)> {
        let mut candidates = Vec::new();
        for (key, methods) in &self.impl_methods {
            if key.self_type_def != self_type_def {
                continue;
            }
            for &method in methods {
                if symbols.get_symbol(method).name == name {
                    candidates.push((method, key.clone()));
                }
            }
        }
        candidates.sort_by_key(|(_, key)| key.trait_id.is_some());
        candidates.into_iter().next()
    }


    pub fn expect_closure_capture_bindings(&self, id: ExprId) -> Vec<CaptureBinding> {
        self.closure_capture_bindings.get(&id).cloned().unwrap_or_else(|| {
            panic!("ICE: closure capture bindings missing for expr {:?}", id)
        })
    }

    /// A trait implementation's spelling is private; calls use the trait contract.
    pub fn callable_contract_symbol(&self, symbol: SymbolId, symbols: &SymbolTable) -> SymbolId {
        self.method_impls.get(&symbol).and_then(|key| key.trait_id)
            .and_then(|owner| self.trait_methods.get(&owner))
            .and_then(|methods| methods.iter().find(|&&method|
                symbols.get_symbol(method).name == symbols.get_symbol(symbol).name))
            .copied().unwrap_or(symbol)
    }

    pub fn callable_signature(&self, symbol: SymbolId, symbols: &SymbolTable) -> Option<&CallableSignature> {
        self.callable_signatures.get(&self.callable_contract_symbol(symbol, symbols))
    }

    pub fn new() -> Self {
        Self {
            callable_signatures: HashMap::new(),
            call_argument_bindings: HashMap::new(),
            expr_types: HashMap::new(),
            expr_symbols: HashMap::new(),
            expr_substs: HashMap::new(),
            expr_trait_resolutions: HashMap::new(),
            try_branch_methods: HashMap::new(),
            try_branch_substs: HashMap::new(),
            try_branch_return_types: HashMap::new(),
            try_from_residual_methods: HashMap::new(),
            try_from_residual_substs: HashMap::new(),
            intrinsic_types: HashMap::new(),
            expr_intrinsics: HashMap::new(),
            expr_member_indices: HashMap::new(),
            expr_struct_init_indices: HashMap::new(),
            expr_sizeof_target: HashMap::new(),
            expr_lifetimes: HashMap::new(),
            expr_captures: HashMap::new(),
            closure_capture_bindings: HashMap::new(),
            closure_mutated_captures: HashMap::new(),
            closure_env_types: HashMap::new(),
            closure_env_ptr_types: HashMap::new(),
            dyn_method_indices: HashMap::new(),
            coercions: HashMap::new(),
            try_branches: HashMap::new(),
            for_loop_next: HashMap::new(),
            for_loop_subst: HashMap::new(),
            for_loop_resolutions: HashMap::new(),
            pat_symbols: HashMap::new(),
            pat_types: HashMap::new(),
            decl_symbols: HashMap::new(),
            symbol_decls: HashMap::new(),
            type_symbols: HashMap::new(),
            ast_type_to_semantic: HashMap::new(),
            symbol_types: HashMap::new(),
            type_scopes: HashMap::new(),
            decl_scopes: HashMap::new(),
            ffi_sync_noescape: HashMap::new(),
            extern_functions: HashSet::new(),
            object_backed_functions: HashSet::new(),
            evaluated_comptime: false,
            drop_impls: HashMap::new(),
            generic_param_symbols: HashMap::new(),
            trait_impls: HashMap::new(),
            trait_methods: HashMap::new(),
            struct_fields: HashMap::new(),
            struct_field_names: HashMap::new(),
            trait_bounds: HashMap::new(),
            trait_generic_params: HashMap::new(),
            impl_methods: HashMap::new(),
            method_impls: HashMap::new(),
            method_to_impl_decl: HashMap::new(),
            method_sym_to_impl_decl: HashMap::new(),
            trait_associated_types: HashMap::new(),
            assoc_type_traits: HashMap::new(),
            impl_associated_types: HashMap::new(),
            assoc_type_names: HashMap::new(),
            assoc_type_bounds: HashMap::new(),
            impl_self_types: HashMap::new(),
            checked_impl_headers: HashMap::new(),
            impl_generic_params: HashMap::new(),
            trait_impl_entries: Vec::new(),
            decl_associated_types: HashMap::new(),
            macro_decls: HashMap::new(),
            decl_macros: HashMap::new(),
            function_effects: HashMap::new(),
            unsafe_functions: HashSet::new(),
            fn_lifetime_contracts: HashMap::new(),
            type_lifetime_contracts: HashMap::new(),
            resolved_type_lifetime_contracts: HashMap::new(),
            raw_storage_anchor_contracts: HashMap::new(),
            resolved_raw_storage_anchor_contracts: HashMap::new(),
        }
    }
}
