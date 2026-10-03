use luna_backend::BackendError;
use luna_driver::test_backend_compile;
use luna_mvir::{Module, Function, BasicBlock, Instruction, LabelId, ValueData, ValueId, ValueOrigin, Terminator, GlobalId};
use luna_semantic::{SemanticContext, SemanticTypeId};

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
