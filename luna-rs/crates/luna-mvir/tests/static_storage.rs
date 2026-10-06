use luna_mvir::interp::RuntimeValue;
use luna_mvir::static_data::{StaticData, StaticType, StaticValue};
use luna_mvir::{
    BasicBlock, Function, GlobalId, Instruction, LabelId, Module, MvirInterpreter, Operand,
    Terminator, ValueData, ValueId,
};
use luna_semantic::{BuiltinType, SemanticContext, SemanticType, SemanticTypeId};

fn function(ty: SemanticTypeId, extra: Option<Instruction>) -> Function {
    let mut instructions = vec![Instruction::StaticAddress(StaticData {
        name: "constant.declaration".into(),
        ty: StaticType::Primitive(BuiltinType::I32),
        value: StaticValue::Int(20),
    })];
    if let Some(extra) = extra {
        instructions.push(extra);
    }
    Function {
        name: GlobalId {
            name: "reference".into(),
            symbol_id: None,
        },
        is_extern: false,
        is_async: false,
        arg_count: 0,
        link_name: None,
        param_types: vec![],
        ret_ty: ty,
        lifetime_info: Default::default(),
        blocks: vec![BasicBlock {
            label: LabelId {
                name: "entry".into(),
            },
            insts: (0..instructions.len()).map(|i| ValueId(i as u32)).collect(),
            terminator: Some(Terminator::Ret {
                value: Some(Operand::Value(ValueId(0))),
            }),
        }],
        values: instructions
            .into_iter()
            .map(|i| ValueData::new(i, ty, None))
            .collect(),
    }
}

#[test]
fn immutable_storage_survives_frames_and_is_not_a_user_heap_leak() {
    let mut ctx = SemanticContext::new();
    let ty = ctx.types.intern(SemanticType::Primitive(BuiltinType::I32));
    let module = Module {
        functions: vec![function(ty, None)],
    };
    let mut vm = MvirInterpreter::new(&module, &ctx);
    let first = vm.eval_function(&module.functions[0], vec![]).unwrap();
    let second = vm.eval_function(&module.functions[0], vec![]).unwrap();
    let (RuntimeValue::Pointer(first), RuntimeValue::Pointer(second)) = (first, second) else {
        panic!("storage address required")
    };
    assert_eq!(first, second);
    assert!(vm.heap.allocations.is_empty());
    assert!(vm.call_stack.is_empty());
}

#[test]
fn immutable_storage_rejects_write_free_drop_and_mutable_borrow() {
    let mut ctx = SemanticContext::new();
    let ty = ctx.types.intern(SemanticType::Primitive(BuiltinType::I32));
    let base = Operand::Value(ValueId(0));
    for operation in [
        Instruction::Store {
            ptr: base.clone(),
            value: Operand::Number("21".into()),
        },
        Instruction::HeapFree {
            value: base.clone(),
        },
        Instruction::Drop {
            value: base.clone(),
            ty,
            callee: None,
        },
        Instruction::Borrow {
            base: base.clone(),
            is_rw: true,
        },
    ] {
        let module = Module {
            functions: vec![function(ty, Some(operation))],
        };
        let error = MvirInterpreter::new(&module, &ctx)
            .eval_function(&module.functions[0], vec![])
            .unwrap_err();
        assert!(error.to_string().contains("constant storage"), "{error}");
    }
}
