pub mod resolver;
pub mod typechecker;
pub mod symbol;
pub mod semantic_tables;
pub mod ty;
pub mod mono;
pub mod macro_engine;
pub mod annotation;
pub mod derive;
pub mod comptime;
pub mod effect;
pub mod lifetime;
pub mod lang_item;
pub mod coherence;
pub mod const_eval;
pub mod coercion;
pub mod cast;
mod operators;

pub mod mangler;
pub mod region;

pub use coercion::{CoercionKind, try_coerce};
pub use effect::{Effect, EffectSet};
pub use resolver::Resolver;
pub use resolver::{ModuleNamespaceProvider, ModuleNamespaceMap};
pub use typechecker::TypeChecker;
pub use mangler::Mangler;
pub use mono::{MonoCollector, MonoInstance, InstantiatedFunction, CanonicalInstanceKind, CanonicalInstanceIdentity, canonical_type_mangling};
pub use macro_engine::MacroEngine;
pub use annotation::AttributeProcessor;
pub use derive::{DeriveRegistry, DeriveContext, DeriveInput, DeriveKind};
pub use comptime::{ComptimeValue, ComptimeError, IntWidth, FloatWidth, TypeRepr, TypeInfoStruct, TypeKind, TypeInfoField, TypeInfoVariant};
pub use symbol::{SymbolTable, ScopeId, SymbolKind, ProviderId, ImportSymbolResult};
pub use luna_common::ids::SymbolId;

pub use semantic_tables::{CaptureBinding, CaptureMode, SemanticTables, IntrinsicKind, CallableSignature, CallArgumentBinding};
pub use ty::{TypeContext, SemanticTypeId, SemanticType, BuiltinType};
pub use lifetime::{
    LifetimeIdent, LifetimeVar, LifetimeConstraintExpr, Provenance,
    LifetimeSolver, SolveResult, LifetimeAssignment,
    LifetimeError, resolve_fn_lifetime_signature,
    LIFETIME_RELATION_ABI_VERSION, CanonicalProvenance, CanonicalContractSubject, CanonicalOutlivesConstraint, CanonicalLifetimeContract,
    ResolvedTypeLifetimeSubject, ResolvedTypeOutlivesConstraint, ResolvedTypeLifetimeContract,
    CanonicalFieldPath, CanonicalTypeLifetimeSubject, CanonicalTypeOutlivesConstraint, CanonicalTypeLifetimeContract,
    CanonicalRawStorageAnchorContract, ResolvedRawStorageAnchorContract,
    LifetimeObligation,
};

pub trait ComptimeEngine: Send + Sync {
    fn eval_expr(&self, arena: &luna_ast::AstArena, ctx: &SemanticContext, source_manager: &luna_common::source::SourceManager, expr_id: luna_ast::ExprId) -> Result<ComptimeValue, ComptimeError>;
    fn eval_stmt(&self, arena: &luna_ast::AstArena, ctx: &SemanticContext, source_manager: &luna_common::source::SourceManager, stmt_id: luna_ast::StmtId) -> Result<ComptimeValue, ComptimeError>;
}

use luna_ast::AstArena;
use luna_common::Diagnostic;

use std::cell::RefCell;
use std::collections::HashMap;

#[cfg(test)]
mod validity_borrow_shape_tests {
    use super::*;
    #[test]
    fn anchored_headers_transport_loans_without_becoming_raw_origins() {
        let mut ctx = SemanticContext::new();
        let byte = ctx.types.intern(SemanticType::Primitive(BuiltinType::U8));
        let pointer = ctx.types.intern(SemanticType::Pointer(ty::Mutability::Immutable, byte));
        let unanchored = ctx.types.intern(SemanticType::Struct(SymbolId(100), vec![], vec![pointer]));
        let anchored = ctx.types.intern(SemanticType::Struct(SymbolId(101), vec![], vec![pointer]));
        ctx.tables.raw_storage_anchor_contracts.insert(SymbolId(101), CanonicalRawStorageAnchorContract::new(vec!["data".into()]));
        let aggregate = ctx.types.intern(SemanticType::Enum(SymbolId(102), vec![], vec![anchored]));
        assert!(!ctx.may_carry_validity_borrow(pointer));
        assert!(!ctx.may_carry_validity_borrow(unanchored));
        assert!(ctx.may_carry_validity_borrow(anchored));
        assert!(ctx.may_carry_validity_borrow(aggregate));
        assert!(!ctx.types.contains_safe_reference(anchored));
        assert!(!ctx.types.contains_safe_reference(aggregate));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NeedsDropState {
    Visiting,
    Yes,
    No,
}

#[derive(Clone)]
pub struct SemanticContext {
    /// Pointer width of the compilation target. The native-only driver defaults
    /// to its native width; explicit semantic target contexts may override it.
    pub target_pointer_bits: u32,
    pub symbol_table: SymbolTable,
    pub tables: SemanticTables,
    pub types: TypeContext,
    pub instantiated_functions: Vec<InstantiatedFunction>,
    pub drop_glue_instances: Vec<mono::CanonicalInstanceIdentity>,
    /// Canonical closure-value ABI: a pointer to a two-pointer { code, env }
    /// record. Structural and fixed, independent of any closure expression.
    pub closure_value_ptr_ty: Option<ty::SemanticTypeId>,
    /// Scoped semantic preparation for early evaluation; never artifact metadata.
    pub comptime_root: Option<mono::MonoRoot>,
    pub diagnostics: Vec<Diagnostic>,
    pub lang_items: lang_item::LangItemRegistry,
    pub needs_drop_cache: RefCell<HashMap<ty::SemanticTypeId, NeedsDropState>>,
    pub comptime_values: HashMap<luna_ast::ExprId, comptime::ComptimeValue>,
    pub const_values: HashMap<SymbolId, comptime::ComptimeValue>,
    pub allow_internal_lang_items: bool,
    pub provider_scopes: HashMap<symbol::ProviderId, ScopeId>,
    pub provider_lookup: HashMap<String, symbol::ProviderId>,
    pub external_module_scopes: HashMap<String, ScopeId>,
    pub current_provider: Option<symbol::ProviderId>,
    pub current_provider_name: Option<String>,
    pub is_slice_authorized: bool,
}

impl SemanticContext {
    /// Whether a value can transport an already established validity borrow.
    /// An anchored header may be an owning value or a borrowed view. Its
    /// contract permits transport of an incoming loan; it never creates one
    /// from a raw address or proves allocation ownership on its own.
    pub fn may_carry_validity_borrow(&self, ty: SemanticTypeId) -> bool {
        fn visit(ctx: &SemanticContext, ty: SemanticTypeId, seen: &mut std::collections::HashSet<SemanticTypeId>) -> bool {
            let ty = ctx.types.resolve(ty);
            if !seen.insert(ty) { return false; }
            match ctx.types.get(ty) {
                SemanticType::Reference(..) => true,
                SemanticType::Struct(symbol, _, fields) => {
                    ctx.tables.raw_storage_anchor_contracts.get(symbol).is_some_and(|contract| !contract.is_empty())
                        || fields.iter().any(|field| visit(ctx, *field, seen))
                }
                SemanticType::Enum(_, _, fields) | SemanticType::Tuple(fields) => fields.iter().any(|field| visit(ctx, *field, seen)),
                SemanticType::Array(element, _) | SemanticType::Future(element) | SemanticType::Range(element) => visit(ctx, *element, seen),
                SemanticType::Closure(_, captures, _) => captures.iter().any(|capture| visit(ctx, *capture, seen)),
                SemanticType::GenericParam(_) | SemanticType::InferenceVar(_) | SemanticType::Projection { .. } => true,
                _ => false,
            }
        }
        visit(self, ty, &mut std::collections::HashSet::new())
    }

    pub fn new() -> Self {
        let mut context = Self {
            // The current driver emits for its native target. Explicit target
            // contexts may override this before semantic analysis starts.
            target_pointer_bits: usize::BITS,
            symbol_table: SymbolTable::new(),
            tables: SemanticTables::new(),
            types: TypeContext::new(),
            lang_items: lang_item::LangItemRegistry::new(),
            instantiated_functions: Vec::new(),
            drop_glue_instances: Vec::new(),
            comptime_root: None,
            diagnostics: Vec::new(),
            needs_drop_cache: RefCell::new(HashMap::new()),
            comptime_values: HashMap::new(),
            const_values: HashMap::new(),
            allow_internal_lang_items: false,
            provider_scopes: HashMap::new(),
            provider_lookup: HashMap::new(),
            external_module_scopes: HashMap::new(),
            current_provider: None,
            current_provider_name: None,
            is_slice_authorized: false,
            closure_value_ptr_ty: None,
        };
        // Canonical closure-value ABI: a pointer to a two-pointer { code, env }
        // record. Fixed shape, independent of any closure expression.
        let usize_ty = context.types.usize_id();
        let abi = context.types.intern(ty::SemanticType::Tuple(vec![usize_ty, usize_ty]));
        context.closure_value_ptr_ty = Some(context.types.intern(ty::SemanticType::Pointer(ty::Mutability::Immutable, abi)));
        context
    }

    pub fn get_symbol_type(&self, sym_id: SymbolId) -> Option<&SemanticType> {
        let ty_id = self.tables.symbol_types.get(&sym_id)?;
        Some(self.types.get(*ty_id))
    }

    pub fn needs_drop(&self, id: ty::SemanticTypeId) -> bool {
        let ty = self.types.get(id);
        if let Some(&state) = self.needs_drop_cache.borrow().get(&id) {
            match state {
                NeedsDropState::Yes => return true,
                NeedsDropState::No => return false,
                NeedsDropState::Visiting => return false, // break cycle safely
            }
        }
        
        self.needs_drop_cache.borrow_mut().insert(id, NeedsDropState::Visiting);
        
        let ty = self.types.get(id);
        let result = match ty {
            ty::SemanticType::Struct(sym_id, _, fields) => {
                let has_drop_impl = self.tables.drop_impls.contains_key(sym_id)
                    || self.lang_items.get(crate::lang_item::LangItem::Drop).map_or(false, |drop_sym| {
                        self.tables.trait_impls.contains_key(&crate::semantic_tables::ImplKey {
                            trait_id: Some(drop_sym),
                            self_type_def: (*sym_id).into(),
                        })
                    });
                if has_drop_impl {
                    true
                } else {
                    fields.iter().any(|&f| self.needs_drop(f))
                }
            }
            ty::SemanticType::Enum(sym_id, _, variants) => {
                let has_drop_impl = self.tables.drop_impls.contains_key(&sym_id)
                    || self.lang_items.get(crate::lang_item::LangItem::Drop).map_or(false, |drop_sym| {
                        self.tables.trait_impls.contains_key(&crate::semantic_tables::ImplKey {
                            trait_id: Some(drop_sym),
                            self_type_def: (*sym_id).into(),
                        })
                    });
                if has_drop_impl {
                    true
                } else {
                    variants.iter().any(|&v| self.needs_drop(v))
                }
            }
            ty::SemanticType::Tuple(fields) => {
                fields.iter().any(|&f| self.needs_drop(f))
            }
            ty::SemanticType::Closure(closure_expr, _, _) => self
                .tables
                .closure_env_types
                .get(closure_expr)
                .copied()
                .map_or(false, |env_ty| self.needs_drop(env_ty)),
            ty::SemanticType::Array(elem_ty, _) => {
                self.needs_drop(*elem_ty)
            }
            ty::SemanticType::Future(_) => true,
            ty::SemanticType::DynTrait(_) => true,
            ty::SemanticType::GenericParam(_) => false, // TODO(phase_1c): Fix generic substitution. Cannot return true yet because it breaks borrow checker for generic enums (e.g., match Option<T> causes conditional move drop errors).
            _ => false,
        };
        
        self.needs_drop_cache.borrow_mut().insert(id, if result { NeedsDropState::Yes } else { NeedsDropState::No });
        result
    }
}
