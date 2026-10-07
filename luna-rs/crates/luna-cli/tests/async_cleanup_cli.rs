//! CLI parity for async cancellation cleanup: source-only and freshly built
//! artifact-only providers must destroy captured owned values exactly once on
//! cancellation, and must not destroy borrowed pointees.
#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn async_cancellation_cleanup_matches_source_and_fresh_artifacts() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/async_cleanup");
    let positives = [
        "value_across_suspend",
        "borrow_only_across_suspend",
        "generic_owned_across_suspend",
    ];
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let result = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(project.join("provider.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(result.status.success(), "provider build: {}", render(&result));
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
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
