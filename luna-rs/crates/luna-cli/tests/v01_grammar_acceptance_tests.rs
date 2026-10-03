#[path = "support/stdlib.rs"]
mod stdlib;

use std::fs;
use std::process::Command;
use stdlib::{render, workspace_root, ProviderModes};

#[test]
fn test_v01_grammar_01_struct_semicolon_rejection() {
    let modes = ProviderModes::fresh();
    let temp = modes.source.join("test_grammar_01");
    fs::create_dir_all(&temp).unwrap();

    // Positive: comma-delimited struct fields
    let positive_src = r#"
export struct Point {
    x: i32,
    y: i32,
};
fn main() -> i32 {
    dec pt = Point { x: 10, y: 32 };
    return pt.x + pt.y - 42;
}
"#;
    let positive_path = temp.join("positive.ln");
    fs::write(&positive_path, positive_src).unwrap();
    let check = modes.check(&modes.source, &positive_path);
    assert!(check.status.success(), "comma fields must compile: {}", render(&check));

    // Negative: semicolon-delimited struct fields
    let negative_src = r#"
export struct BadPoint {
    x: i32;
    y: i32;
};
fn main() -> i32 { return 0; }
"#;
    let negative_path = temp.join("negative.ln");
    fs::write(&negative_path, negative_src).unwrap();
    let bad_check = modes.check(&modes.source, &negative_path);
    assert!(!bad_check.status.success(), "semicolon fields must be rejected");
    let err_msg = render(&bad_check);
    assert!(
        err_msg.contains("error[E0002]") || err_msg.contains("E0002"),
        "expected E0002 diagnostic code for semicolon fields, got: {err_msg}"
    );
    assert!(
        err_msg.contains("semicolons (';') are forbidden"),
        "expected semicolon forbidden message, got: {err_msg}"
    );

    // Negative: mixed comma and semicolon
    let mixed_src = r#"
export struct MixedPoint {
    x: i32,
    y: i32;
};
fn main() -> i32 { return 0; }
"#;
    let mixed_path = temp.join("mixed.ln");
    fs::write(&mixed_path, mixed_src).unwrap();
    let mixed_check = modes.check(&modes.source, &mixed_path);
    assert!(!mixed_check.status.success(), "mixed semicolon fields must be rejected");
    let mixed_err = render(&mixed_check);
    assert!(
        mixed_err.contains("error[E0002]") || mixed_err.contains("E0002"),
        "expected E0002 diagnostic code for mixed fields, got: {mixed_err}"
    );
}

#[test]
fn test_v01_grammar_02_parenthesized_foreach() {
    let modes = ProviderModes::fresh();
    let temp = modes.source.join("test_grammar_02");
    fs::create_dir_all(&temp).unwrap();

    let repo_root = workspace_root().parent().unwrap().to_path_buf();
    let fixture = repo_root.join("tests/luna/language/spec_v01/foreach_parenthesized_contract.ln");
    let target = temp.join("foreach_parenthesized.ln");
    fs::copy(&fixture, &target).unwrap();

    let check = modes.check(&modes.source, &target);
    assert!(check.status.success(), "parenthesized foreach must check: {}", render(&check));

    let (exe, build) = modes.build(&modes.source, &target, "foreach_paren_exe");
    assert!(build.status.success(), "build failed: {}", render(&build));
    let run = Command::new(exe).output().unwrap();
    assert_eq!(run.status.code(), Some(0), "run failed: {}", render(&run));

    // Characterization: legacy unparenthesized foreach also works
    let legacy_fixture = repo_root.join("tests/luna/language/spec_v01/foreach_current_parser.ln");
    let legacy_target = temp.join("foreach_legacy.ln");
    fs::copy(&legacy_fixture, &legacy_target).unwrap();

    let (legacy_exe, legacy_build) = modes.build(&modes.source, &legacy_target, "foreach_legacy_exe");
    assert!(legacy_build.status.success(), "legacy build failed: {}", render(&legacy_build));
    let legacy_run = Command::new(legacy_exe).output().unwrap();
    assert_eq!(legacy_run.status.code(), Some(0), "legacy run failed: {}", render(&legacy_run));
}

#[test]
fn test_v01_grammar_03_receiver_shorthand_acceptance() {
    let modes = ProviderModes::fresh();
    let temp = modes.source.join("test_grammar_03");
    fs::create_dir_all(&temp).unwrap();

    let repo_root = workspace_root().parent().unwrap().to_path_buf();
    let fixture = repo_root.join("tests/luna/language/spec_v01/receiver_shorthand_contract.ln");
    let target = temp.join("receiver_shorthand.ln");
    fs::copy(&fixture, &target).unwrap();

    let (exe, build) = modes.build(&modes.source, &target, "receiver_shorthand_exe");
    assert!(build.status.success(), "build failed: {}", render(&build));
    let run = Command::new(exe).output().unwrap();
    assert_eq!(run.status.code(), Some(0), "run failed: {}", render(&run));

    // Test mutable reference receiver &rw self and value receiver self
    let receivers_src = r#"
struct Accumulator {
    value: i32,
};

impl Accumulator {
    fn read(&self) -> i32 {
        return self.value;
    }

    fn add(&rw self, n: i32) {
        self.value = self.value + n;
    }

    fn into_value(self) -> i32 {
        return self.value;
    }
}

fn main() -> i32 {
    dec rw acc = Accumulator { value: 10 };
    acc.add(32);
    if acc.read() != 42 {
        return 1;
    }
    dec final_val = acc.into_value();
    if final_val != 42 {
        return 2;
    }
    return 0;
}
"#;
    let receivers_path = temp.join("all_receivers.ln");
    fs::write(&receivers_path, receivers_src).unwrap();
    let (acc_exe, acc_build) = modes.build(&modes.source, &receivers_path, "all_receivers_exe");
    assert!(acc_build.status.success(), "all_receivers build failed: {}", render(&acc_build));
    let acc_run = Command::new(acc_exe).output().unwrap();
    assert_eq!(acc_run.status.code(), Some(0), "all_receivers run failed: {}", render(&acc_run));

    // Negative: duplicate receiver
    let dup_src = r#"
struct DupReceiver {};
impl DupReceiver {
    fn bad(&self, &self) {}
}
fn main() -> i32 { return 0; }
"#;
    let dup_path = temp.join("dup_receiver.ln");
    fs::write(&dup_path, dup_src).unwrap();
    let dup_check = modes.check(&modes.source, &dup_path);
    assert!(!dup_check.status.success(), "duplicate receiver must be rejected");
    let dup_err = render(&dup_check);
    assert!(
        dup_err.contains("Receiver parameter 'self' must be the first parameter"),
        "expected non-first receiver message, got: {dup_err}"
    );
}

#[test]
fn test_v01_diag_01_typed_diagnostics_rendered() {
    let modes = ProviderModes::fresh();
    let temp = modes.source.join("test_diag_01");
    fs::create_dir_all(&temp).unwrap();

    // 1. Parser error format with stable diagnostic codes like error[E0001]
    let missing_brace_src = r#"
fn broken(x: i32 -> i32 { return x; }
"#;
    let missing_path = temp.join("missing_paren.ln");
    fs::write(&missing_path, missing_brace_src).unwrap();
    let check = modes.check(&modes.source, &missing_path);
    assert!(!check.status.success());
    let err = render(&check);
    assert!(
        err.contains("error[E0001]") || err.contains("error[E0002]") || err.contains("error[E0003]"),
        "expected typed diagnostic error[E000X], got: {err}"
    );

    // 2. Semantic typechecker error with typed code error[E2026] (InvalidCast)
    let invalid_char_src = r#"
fn main() -> i32 {
    dec invalid: char = 55296 as char;
    return 0;
}
"#;
    let invalid_char_path = temp.join("invalid_char.ln");
    fs::write(&invalid_char_path, invalid_char_src).unwrap();
    let check_char = modes.check(&modes.source, &invalid_char_path);
    assert!(!check_char.status.success());
    let err_char = render(&check_char);
    assert!(
        err_char.contains("error[E2026]"),
        "expected typed diagnostic error[E2026] for invalid char cast, got: {err_char}"
    );

    // 3. Semantic coherence error with typed code error[E2006] (OrphanImpl)
    let orphan_src = r#"
impl<T> [T] {
    fn user_hack(self: &[T]) -> i32 { return 0; }
}
fn main() -> i32 { return 0; }
"#;
    let orphan_path = temp.join("orphan.ln");
    fs::write(&orphan_path, orphan_src).unwrap();
    let check_orphan = modes.check(&modes.source, &orphan_path);
    assert!(!check_orphan.status.success());
    let err_orphan = render(&check_orphan);
    assert!(
        err_orphan.contains("error[E2006]"),
        "expected typed diagnostic error[E2006] for orphan impl, got: {err_orphan}"
    );

    // 4. Type mismatch on variable assignment: error[E2001]
    let assign_mismatch_src = r#"
fn main() -> i32 {
    dec x: i32 = true;
    return 0;
}
"#;
    let assign_path = temp.join("assign_mismatch.ln");
    fs::write(&assign_path, assign_mismatch_src).unwrap();
    let check_assign = modes.check(&modes.source, &assign_path);
    assert!(!check_assign.status.success());
    let err_assign = render(&check_assign);
    assert!(
        err_assign.contains("error[E2001]"),
        "expected typed diagnostic error[E2001] for assign mismatch, got: {err_assign}"
    );

    // 5. Type mismatch on call argument: error[E2001]
    let arg_mismatch_src = r#"
fn foo(x: i32) -> i32 {
    return x;
}
fn main() -> i32 {
    foo(true);
    return 0;
}
"#;
    let arg_path = temp.join("arg_mismatch.ln");
    fs::write(&arg_path, arg_mismatch_src).unwrap();
    let check_arg = modes.check(&modes.source, &arg_path);
    assert!(!check_arg.status.success());
    let err_arg = render(&check_arg);
    assert!(
        err_arg.contains("error[E2001]"),
        "expected typed diagnostic error[E2001] for arg mismatch, got: {err_arg}"
    );

    // 6. Unresolved method on struct: error[E1001]
    let method_not_found_src = r#"
struct Point {
    x: i32,
    y: i32,
};
fn main() -> i32 {
    dec p = Point { x: 1, y: 2 };
    p.non_existent();
    return 0;
}
"#;
    let method_path = temp.join("method_not_found.ln");
    fs::write(&method_path, method_not_found_src).unwrap();
    let check_method = modes.check(&modes.source, &method_path);
    assert!(!check_method.status.success());
    let err_method = render(&check_method);
    assert!(
        err_method.contains("error[E1001]"),
        "expected typed diagnostic error[E1001] for unresolved method, got: {err_method}"
    );

    // 7. Slice inherent authorization: file named slice.ln cannot impersonate sysroot: error[E2006]
    let slice_impersonate_src = r#"
impl<T> [T] {
    fn impersonate(self: &[T]) -> i32 { return 0; }
}
fn main() -> i32 { return 0; }
"#;
    let slice_impersonate_path = temp.join("slice.ln");
    fs::write(&slice_impersonate_path, slice_impersonate_src).unwrap();
    let check_impersonate = modes.check(&modes.source, &slice_impersonate_path);
    assert!(!check_impersonate.status.success());
    let err_impersonate = render(&check_impersonate);
    assert!(
        err_impersonate.contains("error[E2006]"),
        "expected typed diagnostic error[E2006] for slice impersonation, got: {err_impersonate}"
    );
}

#[test]
fn test_v01_grammar_06_a01_slice_authorization_impersonation_prevention() {
    let modes = ProviderModes::fresh();
    let temp = modes.source.join("test_grammar_06");
    fs::create_dir_all(&temp).unwrap();

    // 1. Positive: Official sysroot slice provider works in source mode with .iter()
    let pos_slice_src = r#"
import <slice>;

fn total(xs: &[i32]) -> i32 {
    dec rw sum = 0;
    dec rw it = xs.iter();
    dec rw going = true;
    while going {
        match it.next() {
            std::Option::Some(item) -> { sum = sum + *item; },
            std::Option::None -> { going = false; },
        }
    }
    return sum;
}

fn bump(xs: &rw [i32]) -> i32 {
    dec rw it = xs.iter_mut();
    dec rw going = true;
    while going {
        match it.next() {
            std::Option::Some(item) -> { *item = *item + 1; },
            std::Option::None -> { going = false; },
        }
    }
    return 0;
}

fn main() -> i32 {
    dec rw arr = [10, 20, 30];
    if total(&arr) != 60 { return 1; }
    bump(&rw arr);
    if total(&arr) != 63 { return 2; }
    return 0;
}
"#;
    let pos_path = temp.join("pos_slice.ln");
    fs::write(&pos_path, pos_slice_src).unwrap();
    let (exe_src, build_src) = modes.build(&modes.source, &pos_path, "pos_slice_src_exe");
    assert!(build_src.status.success(), "source slice iter build failed: {}", render(&build_src));
    let run_src = Command::new(exe_src).output().unwrap();
    assert_eq!(run_src.status.code(), Some(0));

    // 2. Positive: Fresh artifact mode consumer runs without source fallback
    let (exe_art, build_art) = modes.build(&modes.artifact, &pos_path, "pos_slice_art_exe");
    assert!(build_art.status.success(), "artifact slice iter build failed: {}", render(&build_art));
    let run_art = Command::new(exe_art).output().unwrap();
    assert_eq!(run_art.status.code(), Some(0));

    // 3. Positive: User-defined nominal type has inherent impl
    let pos_nominal_src = r#"
struct MyPoint {
    x: i32,
    y: i32,
};
impl MyPoint {
    fn sum(self: &MyPoint) -> i32 {
        return self.x + self.y;
    }
}
fn main() -> i32 {
    dec p = MyPoint { x: 12, y: 30 };
    return p.sum() - 42;
}
"#;
    let pos_nom_path = temp.join("pos_nominal.ln");
    fs::write(&pos_nom_path, pos_nominal_src).unwrap();
    let (exe_nom, build_nom) = modes.build(&modes.source, &pos_nom_path, "pos_nom_exe");
    assert!(build_nom.status.success(), "nominal inherent impl failed: {}", render(&build_nom));
    let run_nom = Command::new(exe_nom).output().unwrap();
    assert_eq!(run_nom.status.code(), Some(0));

    // Common hack snippet trying to define inherent impl on [T]
    let user_hack_src = r#"
impl<T> [T] {
    export fn user_hack(self: &[T]) -> i32 {
        return 0;
    }
}
"#;

    // 4. Negative: User provider named `user_provider.ln` imported via `import "user_provider";` -> reject E2006
    let user_provider_path = temp.join("user_provider.ln");
    fs::write(&user_provider_path, user_hack_src).unwrap();
    let consumer_user_src = r#"
import "user_provider";
fn main() -> i32 {
    return 0;
}
"#;
    let consumer_user_path = temp.join("consumer_user.ln");
    fs::write(&consumer_user_path, consumer_user_src).unwrap();
    let check_user = modes.check(&modes.source, &consumer_user_path);
    assert!(!check_user.status.success(), "import user_provider must be rejected");
    let err_user = render(&check_user);
    assert!(err_user.contains("error[E2006]"), "expected error[E2006] for user_provider, got: {err_user}");

    // 5. Negative: User provider named `slice.ln` imported via `import "slice";` -> MUST reject E2006 (no impersonation)
    let slice_provider_path = temp.join("slice.ln");
    fs::write(&slice_provider_path, user_hack_src).unwrap();
    let consumer_slice_src = r#"
import "slice";
fn main() -> i32 {
    return 0;
}
"#;
    let consumer_slice_path = temp.join("consumer_slice.ln");
    fs::write(&consumer_slice_path, consumer_slice_src).unwrap();
    let check_slice = modes.check(&modes.source, &consumer_slice_path);
    assert!(!check_slice.status.success(), "import slice must be rejected");
    let err_slice = render(&check_slice);
    assert!(err_slice.contains("error[E2006]"), "expected error[E2006] for slice impersonation, got: {err_slice}");

    // 6. Negative: User provider named `core.ln` imported via `import "core";` -> MUST reject E2006 (no impersonation)
    let core_provider_path = temp.join("core.ln");
    fs::write(&core_provider_path, user_hack_src).unwrap();
    let consumer_core_src = r#"
import "core";
fn main() -> i32 {
    return 0;
}
"#;
    let consumer_core_path = temp.join("consumer_core.ln");
    fs::write(&consumer_core_path, consumer_core_src).unwrap();
    let check_core = modes.check(&modes.source, &consumer_core_path);
    assert!(!check_core.status.success(), "import core must be rejected");
    let err_core = render(&check_core);
    assert!(err_core.contains("error[E2006]"), "expected error[E2006] for core impersonation, got: {err_core}");
}

#[test]
fn test_v01_grammar_07_diag_counterexamples_span_and_typed_code() {
    let modes = ProviderModes::fresh();
    let temp = modes.source.join("test_grammar_07");
    fs::create_dir_all(&temp).unwrap();

    // Counterexample A: Missing field on struct -> error[E1001] with primary span
    let missing_field_src = r#"
struct Thing {
    x: i32,
};

fn main() -> i32 {
    dec t = Thing { x: 1 };
    return t.nope;
}
"#;
    let missing_path = temp.join("missing_field.ln");
    fs::write(&missing_path, missing_field_src).unwrap();
    let check_missing = modes.check(&modes.source, &missing_path);
    assert!(!check_missing.status.success());
    let err_missing = render(&check_missing);
    assert!(
        err_missing.contains("error[E1001]"),
        "expected typed diagnostic error[E1001] for missing field, got: {err_missing}"
    );
    assert!(
        err_missing.contains("Unknown field or method 'nope'"),
        "expected Unknown field message, got: {err_missing}"
    );
    // DIAG-2: Check that primary span underline is rendered
    assert!(
        err_missing.contains("return t.nope;") && err_missing.contains("^"),
        "expected primary span underline on missing field, got: {err_missing}"
    );

    // Counterexample B1: break outside loop -> error[E2029] with primary span on break
    let break_outside_src = r#"
fn main() -> i32 {
    break;
    return 0;
}
"#;
    let break_path = temp.join("break_outside.ln");
    fs::write(&break_path, break_outside_src).unwrap();
    let check_break = modes.check(&modes.source, &break_path);
    assert!(!check_break.status.success());
    let err_break = render(&check_break);
    assert!(
        err_break.contains("error[E2029]"),
        "expected typed diagnostic error[E2029] for break outside loop, got: {err_break}"
    );
    assert!(
        err_break.contains("`break` outside of a loop"),
        "expected break outside loop message, got: {err_break}"
    );
    // Primary span underlines `break`
    assert!(
        err_break.contains("break;") && err_break.contains("^^^^^"),
        "expected primary span underline on `break`, got: {err_break}"
    );

    // Counterexample B2: continue outside loop -> error[E2029] with primary span on continue
    let continue_outside_src = r#"
fn main() -> i32 {
    continue;
    return 0;
}
"#;
    let continue_path = temp.join("continue_outside.ln");
    fs::write(&continue_path, continue_outside_src).unwrap();
    let check_continue = modes.check(&modes.source, &continue_path);
    assert!(!check_continue.status.success());
    let err_continue = render(&check_continue);
    assert!(
        err_continue.contains("error[E2029]"),
        "expected typed diagnostic error[E2029] for continue outside loop, got: {err_continue}"
    );
    assert!(
        err_continue.contains("`continue` outside of a loop"),
        "expected continue outside loop message, got: {err_continue}"
    );
    assert!(
        err_continue.contains("continue;") && err_continue.contains("^^^^^^^^"),
        "expected primary span underline on `continue`, got: {err_continue}"
    );
}


