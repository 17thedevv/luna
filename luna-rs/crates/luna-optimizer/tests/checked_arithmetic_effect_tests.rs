use luna_common::ids::SymbolId;
use luna_mvir::{Module, Function, BasicBlock, ValueData, Instruction, Terminator, Operand, ValueId, GlobalId, LabelId};
use luna_optimizer::{PassManager, ConstantFolding, DeadCodeElimination};
use luna_semantic::SemanticTypeId;

#[test]
fn dce_preserves_unused_checked_operations_and_their_operands() {
    let integer = SemanticTypeId(1);
    let mut function = Function {
        name: GlobalId { name: "effects".into(), symbol_id: Some(SymbolId(1)) },
        is_extern: false, is_async: false, arg_count: 0, link_name: None,
        param_types: vec![], ret_ty: SemanticTypeId(0), lifetime_info: Default::default(),
        blocks: vec![], values: vec![
            ValueData::new(Instruction::Assign(Operand::Number("2147483647".into())), integer, None),
            ValueData::new(Instruction::Assign(Operand::Number("1".into())), integer, None),
        ],
    };
    let left = Operand::Value(ValueId(0));
    let right = Operand::Value(ValueId(1));
    for instruction in [
        Instruction::Add { left: left.clone(), right: right.clone() },
        Instruction::Sub { left: left.clone(), right: right.clone() },
        Instruction::Mul { left: left.clone(), right: right.clone() },
        Instruction::Div { left: left.clone(), right: right.clone() },
        Instruction::Rem { left: left.clone(), right: right.clone() },
        Instruction::Shl { left: left.clone(), right: right.clone() },
        Instruction::Shr { left: left.clone(), right: right.clone() },
        Instruction::Neg { value: left.clone() },
    ] {
        function.values.push(ValueData::new(instruction, integer, None));
    }
    for ty in [SemanticTypeId(2), SemanticTypeId(3)] { // Float and pointer facts.
        function.values.push(ValueData::new(Instruction::Add {
            left: left.clone(), right: right.clone(),
        }, ty, None));
    }
    function.blocks.push(BasicBlock {
        label: LabelId { name: "entry".into() },
        insts: (0..12).map(ValueId).collect(), terminator: Some(Terminator::Ret { value: None }),
    });
    let mut module = Module::new();
    module.functions.push(function);
    let mut passes = PassManager::new();
    passes.add_pass(Box::new(DeadCodeElimination::with_integer_types([1])));
    passes.run(&mut module);
    assert_eq!(module.functions[0].blocks[0].insts, (0..10).map(ValueId).collect::<Vec<_>>());
}

#[test]
fn folded_safe_arithmetic_is_removable_but_narrow_overflow_is_not() {
    let integer = SemanticTypeId(1);
    let mut function = Function {
        name: GlobalId { name: "fold_then_dce".into(), symbol_id: None },
        is_extern: false, is_async: false, arg_count: 0, link_name: None,
        param_types: vec![], ret_ty: SemanticTypeId(0), lifetime_info: Default::default(),
        blocks: vec![BasicBlock { label: LabelId { name: "entry".into() },
            insts: vec![ValueId(0), ValueId(1)], terminator: Some(Terminator::Ret { value: None }) }],
        values: vec![
            ValueData::new(Instruction::Add { left: Operand::Number("1".into()), right: Operand::Number("2".into()) }, integer, None),
            ValueData::new(Instruction::Add { left: Operand::Number("127".into()), right: Operand::Number("1".into()) }, integer, None),
        ],
    };
    let mut module = Module::new();
    module.functions.push(function);
    let mut passes = PassManager::new();
    passes.add_pass(Box::new(ConstantFolding::with_integer_ranges([(1, 8, true)])));
    passes.add_pass(Box::new(DeadCodeElimination::with_integer_types([1])));
    passes.run(&mut module);
    assert_eq!(module.functions[0].blocks[0].insts, vec![ValueId(1)]);
    assert!(matches!(module.functions[0].values[1].inst, Instruction::Add { .. }));
}
