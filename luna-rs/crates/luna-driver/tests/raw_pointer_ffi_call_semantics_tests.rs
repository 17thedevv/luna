use luna_driver::sysroot::Sysroot;
use luna_driver::{check, CompilerOptions};

fn test_check(name: &str, source: &str) -> Result<(), Vec<luna_common::Diagnostic>> {
    let sysroot = Sysroot::discover_for_test().expect("locate test sysroot");
    let options = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };
    check(name, source.to_string(), &options)
}

fn assert_borrow_conflict_at_call(
    result: Result<(), Vec<luna_common::Diagnostic>>,
    case: &str,
    source: &str,
    callee: &str,
) {
    let errors = result.expect_err(case);
    let conflict = errors.iter().find(|diagnostic| diagnostic.code == Some(luna_common::DiagnosticCode::BorrowConflict))
        .unwrap_or_else(|| panic!("{case} should reject with E3003 BorrowConflict, got: {errors:?}"));
    let call_start = source.rfind(callee).expect("fixture contains the expected extern call") as u32;
    let span = conflict.span.expect("call-scoped FFI conflict should retain a source span");
    assert!(
        span.start >= call_start && span.start <= call_start + callee.len() as u32,
        "{case} must reject at the raw-pointer FFI call, not at borrow creation; span={span:?}, call starts at {call_start}"
    );
}

#[test]
fn raw_ffi_write_ends_temporary_raw_creation_borrow() {
    let out_parameter = include_str!("../../../tests/luna/compiler/raw_out_parameter_after_ffi.ln");
    let result = test_check("raw_ffi_out_parameter", out_parameter);
    assert!(result.is_ok(), "a local must be readable after a call-scoped *rw extern access: {result:?}");

    let raw_stays_live = include_str!("../../../tests/luna/compiler/raw_ffi_pointer_live_after_call_does_not_keep_safe_loan.ln");
    let result = test_check("raw_ffi_pointer_live_after_call", raw_stays_live);
    assert!(result.is_ok(), "a raw pointer that remains live after the call must not keep its creation loan alive: {result:?}");

    let local_unsafe = include_str!("../../../tests/luna/compiler/raw_local_unsafe_write.ln");
    let result = test_check("raw_local_unsafe_write", local_unsafe);
    assert!(result.is_ok(), "the existing direct unsafe raw write control must remain green: {result:?}");
}

#[test]
fn raw_ffi_access_conflicts_with_live_safe_loans() {
    let shared = include_str!("../../../tests/luna/compiler/raw_ffi_active_shared_loan_rejected.ln");
    assert_borrow_conflict_at_call(
        test_check("raw_ffi_active_shared_loan", shared),
        "*rw extern mutation overlapping a live shared loan",
        shared,
        "__test_ffi_write_raw",
    );

    let mutable = include_str!("../../../tests/luna/compiler/raw_ffi_active_mut_loan_rejected.ln");
    assert_borrow_conflict_at_call(
        test_check("raw_ffi_active_mut_loan", mutable),
        "*T extern read overlapping a live mutable loan",
        mutable,
        "__test_ffi_read_raw",
    );
}

#[test]
fn raw_ffi_call_accepts_when_safe_loan_dies_before_call() {
    let source = include_str!("../../../tests/luna/compiler/raw_ffi_loan_ends_before_call.ln");
    let result = test_check("raw_ffi_loan_ends_before_call", source);
    assert!(result.is_ok(), "an NLL-dead shared loan must not block a later *rw extern call: {result:?}");
}

#[test]
fn ffi_written_raw_pointer_value_becomes_unknown() {
    let source = include_str!("../../../tests/luna/compiler/raw_ffi_pointer_out_becomes_unknown.ln");
    let result = test_check("raw_ffi_pointer_out_becomes_unknown", source);
    assert!(result.is_ok(), "a raw pointer written through an extern out parameter must be reloaded as Unknown, not the old null/slot origin: {result:?}");
}

#[test]
fn multiple_scalar_and_read_file_style_outputs_are_accepted() {
    let multiple = include_str!("../../../tests/luna/compiler/raw_ffi_multiple_scalar_out_parameters.ln");
    let result = test_check("raw_ffi_multiple_outputs", multiple);
    assert!(result.is_ok(), "multiple scalar output pointers should be accepted: {result:?}");

    let read_file_style = include_str!("../../../tests/luna/compiler/raw_ffi_read_file_style_outputs.ln");
    let result = test_check("raw_ffi_read_file_style_outputs", read_file_style);
    assert!(result.is_ok(), "read-file-style output pointers should be accepted: {result:?}");
}
