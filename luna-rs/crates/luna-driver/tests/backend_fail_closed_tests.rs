use luna_backend::BackendError;
use luna_driver::test_backend_compile;
use luna_mvir::{Module, Function, BasicBlock, Instruction, LabelId, ValueData, ValueId, ValueOrigin, Terminator, GlobalId};
use luna_semantic::{SemanticContext, SemanticTypeId};

fn drop_module(callee: &str) -> Module {
    Module { functions: vec![Function {
        name: GlobalId { name: "drop_test".into(), symbol_id: None },
        is_extern: false, is_async: false, arg_count: 0, link_name: None,
        param_types: vec![], ret_ty: SemanticTypeId(0),
        blocks: vec![BasicBlock {
            label: LabelId { name: "entry".into() },
            insts: vec![ValueId(0), ValueId(1)],
            terminator: Some(Terminator::Ret { value: None }),
        }],
        values: vec![
            ValueData::new(Instruction::Alloca, SemanticTypeId(3), None),
            ValueData::new(Instruction::Drop {
                value: luna_mvir::Operand::Value(ValueId(0)),
                callee: Some(GlobalId { name: callee.into(), symbol_id: None }),
                ty: SemanticTypeId(3),
            }, SemanticTypeId(3), None),
        ],
    }] }
}

#[test]
fn missing_drop_callee_fails_closed() {
    let result = test_backend_compile(&drop_module("missing_drop_glue"), &SemanticContext::new());
    assert!(matches!(result, Err(BackendError::InvariantViolation(message))
        if message.contains("Drop callee not found: missing_drop_glue")));
}

#[test]
fn valid_external_drop_declaration_is_called() {
    let mut ctx = SemanticContext::new();
    let ptr_ty = ctx.types.intern(luna_semantic::SemanticType::Pointer(
        luna_semantic::ty::Mutability::Mutable, SemanticTypeId(3),
    ));
    let mut module = drop_module("external_drop_glue");
    module.functions.push(Function {
        name: GlobalId { name: "external_drop_glue".into(), symbol_id: None },
        is_extern: true, is_async: false, arg_count: 1, link_name: None,
        param_types: vec![ptr_ty], ret_ty: SemanticTypeId(0), blocks: vec![], values: vec![],
    });
    test_backend_compile(&module, &ctx).expect("a declared external destructor remains valid");
}

#[test]
fn invalid_mvir_is_an_error_at_both_verification_stages() {
    let mut module = drop_module("unused");
    module.functions[0].blocks[0].terminator = None;
    for stage in ["pre-opt", "post-opt"] {
        let errors = luna_driver::validate_mvir_module(&module, stage).unwrap_err();
        assert!(errors.iter().any(|error| error.code == Some(luna_common::DiagnosticCode::BackendInvariantViolation)
            && error.message.contains("missing a terminator")));
    }
}

#[test]
fn test_unhandled_instruction_fails_closed() {
    let semantic_ctx = SemanticContext::new();

    let func = Function {
        name: GlobalId { name: "test_fn".into(), symbol_id: None },
        is_extern: false,
        is_async: false,
        arg_count: 0,
        link_name: None,
        param_types: vec![],
        ret_ty: SemanticTypeId(0),
        blocks: vec![
            BasicBlock {
                label: LabelId { name: "entry".into() },
                insts: vec![ValueId(0)],
                terminator: Some(Terminator::Ret { value: None }),
            }
        ],
        values: vec![
            ValueData {
                inst: Instruction::CallIntrinsic {
                    kind: luna_semantic::semantic_tables::IntrinsicKind::TypeOf,
                    args: vec![],
                },
                ty: SemanticTypeId(0),
                span: None,
                origin: ValueOrigin::Temporary,
            }
        ],
    };

    let module = Module {
        functions: vec![func],
    };

    let result = test_backend_compile(&module, &semantic_ctx);
    assert!(result.is_err(), "Expected compilation to fail closed");
    match result.unwrap_err() {
        BackendError::InvariantViolation(msg) => {
            assert!(
                msg.contains("Unhandled MVIR instruction"),
                "Error message should mention unhandled instruction: {}",
                msg
            );
        }
        other => panic!("Expected InvariantViolation, got: {:?}", other),
    }
}
