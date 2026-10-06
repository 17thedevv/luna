//! Validate compiler-generated raw-slice thunks before applying their unsafe
//! declaration contract. Ordinary Luna bodies never qualify for this boundary.
use luna_common::{Diagnostic, DiagnosticCode};
use luna_mvir::{Function, Instruction, Operand, Terminator, ValueId, ValueOrigin};
use luna_semantic::{
    lang_item::LangItem, ty::Mutability, BuiltinType, SemanticContext, SemanticType,
};

pub(super) fn validate(func: &Function, ctx: &SemanticContext) -> Option<Result<(), Diagnostic>> {
    let item = ctx.lang_items.from_symbol(func.name.symbol_id?);
    let mutable = match item? {
        LangItem::SliceFromRawParts => Mutability::Immutable,
        LangItem::SliceFromRawPartsMut => Mutability::Mutable,
        _ => return None,
    };
    let valid = (|| {
        if func.is_extern
            || func.is_async
            || func.arg_count != 2
            || func.param_types.len() != 2
            || func.blocks.len() != 1
            || func.values.len() != 5
        {
            return false;
        }
        let SemanticType::Pointer(pointer_mut, element) = ctx.types.get(func.param_types[0]) else {
            return false;
        };
        let SemanticType::Reference(_, reference_mut, slice) = ctx.types.get(func.ret_ty) else {
            return false;
        };
        if *pointer_mut != mutable
            || *reference_mut != mutable
            || !matches!(ctx.types.get(*slice), SemanticType::Slice(inner) if inner == element)
            || !matches!(
                ctx.types.get(func.param_types[1]),
                SemanticType::Primitive(BuiltinType::U64)
            )
        {
            return false;
        }
        let block = &func.blocks[0];
        if block.insts != (0..5).map(ValueId).collect::<Vec<_>>()
            || block.terminator
                != Some(Terminator::Ret {
                    value: Some(Operand::Value(ValueId(4))),
                })
        {
            return false;
        }
        for index in 0..2 {
            if !matches!(func.values[index].inst, Instruction::Alloca)
                || func.values[index].origin != ValueOrigin::Parameter(index as u32)
                || func.values[index].ty != func.param_types[index]
                || !matches!(&func.values[index + 2].inst,
                    Instruction::Load { ptr: Operand::Value(source) } if *source == ValueId(index as u32))
                || func.values[index + 2].ty != func.param_types[index]
            {
                return false;
            }
        }
        matches!(&func.values[4].inst, Instruction::MakeSlice {
            data_ptr: Operand::Value(data), len: Operand::Value(length),
        } if *data == ValueId(2) && *length == ValueId(3))
            && func.values[4].ty == func.ret_ty
    })();
    Some(if valid {
        Ok(())
    } else {
        Err(
            Diagnostic::error("invalid compiler-generated raw-slice intrinsic thunk")
                .with_code(DiagnosticCode::BackendInvariantViolation),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use luna_common::ids::SymbolId;
    use luna_mvir::{BasicBlock, GlobalId, LabelId, ValueData};

    fn thunk(mutable: Mutability) -> (Function, SemanticContext) {
        let mut ctx = SemanticContext::new();
        let element = ctx.types.intern(SemanticType::Primitive(BuiltinType::I32));
        let length = ctx.types.intern(SemanticType::Primitive(BuiltinType::U64));
        let pointer = ctx
            .types
            .intern(SemanticType::Pointer(mutable.clone(), element));
        let slice = ctx.types.intern(SemanticType::Slice(element));
        let result = ctx.types.intern(SemanticType::Reference(
            luna_semantic::ty::LifetimeId(0),
            mutable.clone(),
            slice,
        ));
        let item = if mutable == Mutability::Mutable {
            LangItem::SliceFromRawPartsMut
        } else {
            LangItem::SliceFromRawParts
        };
        ctx.lang_items.inject_raw(item, SymbolId(77));
        let func = Function {
            name: GlobalId {
                name: "renamed_primitive".into(),
                symbol_id: Some(SymbolId(77)),
            },
            is_extern: false,
            is_async: false,
            arg_count: 2,
            link_name: None,
            param_types: vec![pointer, length],
            ret_ty: result,
            lifetime_info: Default::default(),
            values: vec![
                ValueData::new(Instruction::Alloca, pointer, None)
                    .with_origin(ValueOrigin::Parameter(0)),
                ValueData::new(Instruction::Alloca, length, None)
                    .with_origin(ValueOrigin::Parameter(1)),
                ValueData::new(
                    Instruction::Load {
                        ptr: Operand::Value(ValueId(0)),
                    },
                    pointer,
                    None,
                ),
                ValueData::new(
                    Instruction::Load {
                        ptr: Operand::Value(ValueId(1)),
                    },
                    length,
                    None,
                ),
                ValueData::new(
                    Instruction::MakeSlice {
                        data_ptr: Operand::Value(ValueId(2)),
                        len: Operand::Value(ValueId(3)),
                    },
                    result,
                    None,
                ),
            ],
            blocks: vec![BasicBlock {
                label: LabelId {
                    name: "entry".into(),
                },
                insts: (0..5).map(ValueId).collect(),
                terminator: Some(Terminator::Ret {
                    value: Some(Operand::Value(ValueId(4))),
                }),
            }],
        };
        (func, ctx)
    }

    #[test]
    fn only_exact_typed_primitive_recipes_receive_the_declared_unsafe_boundary() {
        for mutable in [Mutability::Immutable, Mutability::Mutable] {
            let (func, ctx) = thunk(mutable);
            assert!(validate(&func, &ctx).unwrap().is_ok());
            let mut other = func.clone();
            other.name.symbol_id = Some(SymbolId(78));
            assert!(validate(&other, &ctx).is_none());
            for alteration in 0..7 {
                let mut changed = func.clone();
                match alteration {
                    0 => changed.values[0].origin = ValueOrigin::Local,
                    1 => changed.values[2].inst = Instruction::Assign(Operand::Number("0".into())),
                    2 => changed.blocks[0].terminator = Some(Terminator::Unreachable),
                    3 => changed.blocks[0].insts.swap(2, 3),
                    4 => changed.param_types.swap(0, 1),
                    5 => changed.ret_ty = changed.param_types[0],
                    _ => {
                        changed.values[4].inst = Instruction::MakeSlice {
                            data_ptr: Operand::Value(ValueId(3)),
                            len: Operand::Value(ValueId(2)),
                        }
                    }
                }
                assert!(
                    validate(&changed, &ctx).unwrap().is_err(),
                    "accepted mutation {alteration}"
                );
                let (diagnostics, _) =
                    crate::borrow_check_function(&changed, &ctx, &Default::default());
                assert_eq!(
                    diagnostics[0].code,
                    Some(DiagnosticCode::BackendInvariantViolation)
                );
            }
        }
    }
}
