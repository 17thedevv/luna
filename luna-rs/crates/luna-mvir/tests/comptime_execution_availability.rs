use luna_common::ids::SymbolId;
use luna_mvir::{
    BasicBlock, Function, GlobalId, Instruction, LabelId, Module, MvirInterpreter, Operand,
    Terminator, ValueData, ValueId,
};
use luna_semantic::{
    effect::{Effect, EffectSet},
    BuiltinType, ComptimeError, SemanticContext, SemanticType, SemanticTypeId,
};

fn global(name: &str) -> GlobalId {
    GlobalId {
        name: name.into(),
        symbol_id: None,
    }
}

fn function(name: &str, ty: SemanticTypeId, instructions: Vec<Instruction>) -> Function {
    let count = instructions.len();
    Function {
        name: global(name),
        is_extern: false,
        is_async: false,
        arg_count: 0,
        link_name: None,
        param_types: Vec::new(),
        ret_ty: ty,
        lifetime_info: Default::default(),
        values: instructions
            .into_iter()
            .map(|i| ValueData::new(i, ty, None))
            .collect(),
        blocks: vec![BasicBlock {
            label: LabelId {
                name: "entry".into(),
            },
            insts: (0..count).map(|i| ValueId(i as u32)).collect(),
            terminator: Some(Terminator::Ret { value: None }),
        }],
    }
}

fn drop_call(ty: SemanticTypeId, value: Operand) -> Instruction {
    Instruction::Drop {
        value,
        ty,
        callee: Some(global("required_drop")),
    }
}

fn run(context: &SemanticContext, functions: Vec<Function>) -> Result<(), ComptimeError> {
    let module = Module { functions };
    MvirInterpreter::new(&module, context)
        .eval_function(&module.functions[0], Vec::new())
        .map(|_| ())
}

#[test]
fn missing_required_destructor_is_not_a_no_op() {
    let context = SemanticContext::new();
    let ty = context.types.void_id();
    assert_eq!(
        run(
            &context,
            vec![function(
                "caller",
                ty,
                vec![drop_call(ty, Operand::Number("1".into()))]
            )]
        ),
        Err(ComptimeError::SymbolNotFound("required_drop".into()))
    );
}

#[test]
fn present_required_destructor_is_executed() {
    let mut context = SemanticContext::new();
    let ty = context
        .types
        .intern(SemanticType::Primitive(BuiltinType::I32));
    let destructor = function(
        "required_drop",
        ty,
        vec![Instruction::Div {
            left: Operand::Number("1".into()),
            right: Operand::Number("0".into()),
        }],
    );
    assert_eq!(
        run(
            &context,
            vec![
                function(
                    "caller",
                    ty,
                    vec![drop_call(ty, Operand::Number("1".into()))]
                ),
                destructor
            ]
        ),
        Err(ComptimeError::DivisionByZero)
    );
}

#[test]
fn extern_destructor_remains_forbidden_with_a_retained_body() {
    let context = SemanticContext::new();
    let ty = context.types.void_id();
    let mut destructor = function("required_drop", ty, Vec::new());
    destructor.is_extern = true;
    assert!(matches!(
        run(
            &context,
            vec![
                function(
                    "caller",
                    ty,
                    vec![drop_call(ty, Operand::Number("1".into()))]
                ),
                destructor
            ]
        ),
        Err(ComptimeError::ForbiddenSideEffect(_))
    ));
}

#[test]
fn effectful_destructor_remains_forbidden_with_a_retained_body() {
    let mut context = SemanticContext::new();
    let ty = context.types.void_id();
    let mut destructor = function("required_drop", ty, Vec::new());
    let symbol = SymbolId(123);
    destructor.name.symbol_id = Some(symbol);
    let mut effects = EffectSet::pure();
    effects.add(Effect::IO);
    context.tables.function_effects.insert(symbol, effects);
    assert!(matches!(
        run(
            &context,
            vec![
                function(
                    "caller",
                    ty,
                    vec![drop_call(ty, Operand::Number("1".into()))]
                ),
                destructor
            ]
        ),
        Err(ComptimeError::ForbiddenSideEffect(_))
    ));
}

#[test]
fn ordinary_missing_callee_and_bodyless_entry_fail_closed() {
    let context = SemanticContext::new();
    let ty = context.types.void_id();
    assert_eq!(
        run(
            &context,
            vec![function(
                "caller",
                ty,
                vec![Instruction::CallDirect {
                    callee: global("missing"),
                    args: Vec::new(),
                }]
            )]
        ),
        Err(ComptimeError::SymbolNotFound("missing".into()))
    );
    let mut entry = function("bodyless", ty, Vec::new());
    entry.blocks.clear();
    assert_eq!(
        run(&context, vec![entry]),
        Err(ComptimeError::SymbolNotFound("bodyless".into()))
    );
}

#[test]
fn uninitialized_storage_does_not_require_a_destructor_call() {
    let context = SemanticContext::new();
    let ty = context.types.void_id();
    assert!(run(
        &context,
        vec![function(
            "caller",
            ty,
            vec![
                Instruction::Alloca,
                drop_call(ty, Operand::Value(ValueId(0)))
            ]
        )]
    )
    .is_ok());
}

#[test]
fn already_dropped_storage_is_not_dropped_again() {
    let context = SemanticContext::new();
    let ty = context.types.void_id();
    let ptr = Operand::Value(ValueId(0));
    let first = Instruction::Drop {
        value: ptr.clone(),
        ty,
        callee: None,
    };
    assert!(run(
        &context,
        vec![function(
            "caller",
            ty,
            vec![
                Instruction::Alloca,
                Instruction::Store {
                    ptr: ptr.clone(),
                    value: Operand::Number("1".into())
                },
                first,
                drop_call(ty, ptr)
            ]
        )]
    )
    .is_ok());
}
