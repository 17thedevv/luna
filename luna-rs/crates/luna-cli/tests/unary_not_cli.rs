//! A6 freeze — unary logical NOT.
//!
//! `!` is a boolean operator: it must evaluate correctly (including double
//! negation, combinators, condition, generic and comptime positions) and reject
//! a non-boolean operand with a single typed E2012 that poisons the value
//! (no cascade, no published executable).
#[path = "support/stdlib.rs"]
mod stdlib;

use std::process::Command;
use stdlib::{render, workspace_root, ProviderModes};

fn count_errors(stderr: &str) -> usize {
    stderr
        .lines()
        .filter(|line| line.starts_with("error[") || line.starts_with("error:"))
        .count()
}

#[test]
fn logical_not_semantics_and_containment_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/unary_not");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for name in [
            "not_true",
            "not_false",
            "double_not",
            "not_and",
            "not_in_condition",
            "not_generic",
            "not_comptime",
        ] {
            let fixture = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &fixture);
            let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            if !check.status.success() || !build.status.success() {
                failures.push(format!(
                    "{mode}/{name}: check {} build {}",
                    render(&check),
                    render(&build)
                ));
                continue;
            }
            let run = Command::new(executable).output().unwrap();
            if run.status.code() != Some(0) {
                failures.push(format!("{mode}/{name} run: {}", render(&run)));
            }
        }

        for name in ["not_invalid_type", "not_poison_no_cascade"] {
            let fixture = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &fixture);
            let check_stderr = String::from_utf8_lossy(&check.stderr).to_string();
            if check.status.code() != Some(1) || !check_stderr.contains("error[E2012]") {
                failures.push(format!("{mode}/{name}/check: {}", render(&check)));
            }
            let errors = count_errors(&check_stderr);
            if errors != 1 {
                failures.push(format!(
                    "{mode}/{name} cascade ({errors} errors): {}",
                    render(&check)
                ));
            }

            let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            if build.status.code() != Some(1)
                || !String::from_utf8_lossy(&build.stderr).contains("error[E2012]")
            {
                failures.push(format!("{mode}/{name}/build: {}", render(&build)));
            }
            if executable.exists() {
                failures.push(format!("{mode}/{name} published an executable"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
