#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{ProviderModes, render, workspace_root};

#[test]
fn generic_owned_values_drop_once_across_control_flow_and_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/generic_drop");
    let positives = [
        "move_return",
        "nested_aggregates",
        "partial_aggregate_cleanup",
        "impl_method_substitution",
        "branches_and_early_return",
        "conditional_initialization",
        "conditional_reinitialization",
        "overwrite",
        "raw_write_and_safe_overwrite",
        "loop_exits",
        "future_initial_cancel",
        "closure_borrow_scoped",
        "closure_capture",
    ];
    let negatives = [
        ("use_after_move", "E3001"),
        ("double_ownership", "E3001"),
        ("copy_drop_conflict", "E2004"),
        ("partial_move_drop_owner", "E3002"),
        ("partial_aggregate_use", "E3001"),
        ("inferred_shared_write", "E2023"),
        ("conditional_use", "E3001"),
        ("closure_borrow_escape", "E3005"),
        ("closure_borrow_local_escape", "E3005"),
        ("closure_borrow_move_escape", "E3005"),
    ];
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("resource.ln");
        fs::copy(fixtures.join("resource.ln"), &provider).unwrap();
        if mode == "artifact" {
            let result = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(project.join("resource.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "provider build: {}",
                render(&result)
            );
            fs::remove_file(&provider).unwrap();
        }
        for name in positives {
            let input = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let (exe, result) = modes.build(root, &input, &format!("{name}_{mode}"));
            if !result.status.success() {
                failures.push(format!("{mode}/{name} build: {}", render(&result)));
                continue;
            }
            let run = Command::new(exe).output().unwrap();
            if !run.status.success() {
                failures.push(format!("{mode}/{name} run: {}", render(&run)));
            } else {
                eprintln!("PASS {mode}/{name}: build and native exit0");
            }
        }
        for (name, code) in negatives {
            let input = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (_, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            for (command, result) in [("check", check), ("build", build)] {
                if result.status.success()
                    || !String::from_utf8_lossy(&result.stderr).contains(&format!("error[{code}]"))
                {
                    failures.push(format!(
                        "{mode}/{name} {command}, expected {code}: {}",
                        render(&result)
                    ));
                } else {
                    eprintln!("PASS {mode}/{name} {command}: {code}");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
