//! Canonical identity invariant for closure-environment drop glue.
//!
//! Two closures with the same structural environment tuple must share one
//! `ClosureDropGlue` symbol, and a different environment representation
//! (move `T` vs borrow `&T`) must produce a different symbol. The symbol never
//! embeds a source expression id or a session-local id.
use luna_semantic::mono::{CanonicalInstanceIdentity, CanonicalInstanceKind};
use luna_semantic::ty::{BuiltinType, Mutability, SemanticType, TypeContext};
use luna_semantic::SymbolTable;

fn identity(env_ty: luna_semantic::SemanticTypeId) -> CanonicalInstanceIdentity {
    CanonicalInstanceIdentity {
        kind: CanonicalInstanceKind::ClosureDropGlue { env_ty },
        subst: Vec::new(),
    }
}

#[test]
fn closure_drop_glue_identity_is_structural_and_expression_independent() {
    let mut types = TypeContext::new();
    let symbols = SymbolTable::new();

    let i32_ty = types.intern(SemanticType::Primitive(BuiltinType::I32));
    let ref_i32 = types.intern(SemanticType::Pointer(Mutability::Immutable, i32_ty));

    // Same structural environment tuple -> one canonical glue identity.
    let env_move = types.intern(SemanticType::Tuple(vec![i32_ty]));
    let a = identity(env_move).symbol_name(&types, &symbols, "closure");
    let b = identity(env_move).symbol_name(&types, &symbols, "closure");
    assert_eq!(a, b, "identical environment tuples must share one symbol");
    assert!(
        a.starts_with("__luna_closure_drop_glue_"),
        "unexpected closure glue symbol: {a}"
    );

    // A move capture of `T` and a borrow capture `&T` have different environment
    // representations, hence different glue identities and destruction plans.
    let env_borrow = types.intern(SemanticType::Tuple(vec![ref_i32]));
    let c = identity(env_borrow).symbol_name(&types, &symbols, "closure");
    assert_ne!(a, c, "move T and borrow &T must not share a closure glue symbol");
}
