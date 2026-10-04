//! Canonical destructor identities shared by ordinary and synthesized cleanup.
use crate::GlobalId;
use luna_common::ids::SymbolId;
use luna_semantic::{CanonicalInstanceIdentity, CanonicalInstanceKind, SemanticContext, SemanticType, SemanticTypeId};

pub fn drop_glue_global_id(ctx: &SemanticContext, ty: SemanticTypeId) -> Option<GlobalId> {
    let ty = ctx.types.resolve(ty);
    if !ctx.needs_drop(ty) { return None; }
    let (symbol, base_name, symbol_id) = match ctx.types.get(ty) {
        SemanticType::Struct(symbol, ..) | SemanticType::Enum(symbol, ..) => {
            let name = ctx.symbol_table.symbols.get(symbol.0 as usize)
                .map(|s| s.name.clone()).unwrap_or_else(|| format!("type{}", symbol.0));
            (*symbol, name, Some(*symbol))
        }
        SemanticType::Tuple(_) => (SymbolId(0), "tuple".into(), None),
        SemanticType::Array(_, _) => (SymbolId(0), "array".into(), None),
        _ => return None,
    };
    let identity = CanonicalInstanceIdentity {
        kind: CanonicalInstanceKind::DropGlue { struct_sym: symbol, concrete_ty: ty },
        subst: Vec::new(),
    };
    Some(GlobalId {
        name: identity.symbol_name(&ctx.types, &ctx.symbol_table, &base_name), symbol_id,
    })
}
