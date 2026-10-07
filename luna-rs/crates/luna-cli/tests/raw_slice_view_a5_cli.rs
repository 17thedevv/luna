//! A5 freeze — raw slice / view contract.
//!
//! Locks the invariant that a raw pointer/slice projection is not an owned
//! aggregate subplace: view conflicts are rejected (E3003) and lifetime escapes
//! (E3005), while valid views, projected views, zero-length slices and slice
//! copies behave; an out-of-bounds slice index traps deterministically.
#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn raw_slice_and_view_contract_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/raw_slice_contract");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(project.join("provider.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{mode}/provider: {}", render(&output));
            fs::remove_file(&provider).unwrap();
        }

        for (name, code) in [
            ("a5_shared_view_rw_conflict", "E3003"),
            ("a5_mut_view_competing_read", "E3003"),
            ("a5_projected_view_conflict", "E3003"),
            ("a5_view_escape_backing_lifetime", "E3005"),
        ] {
            let input = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (_, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            for (command, output) in [("check", check), ("build", build)] {
                if output.status.success()
                    || !String::from_utf8_lossy(&output.stderr).contains(&format!("error[{code}]"))
                {
                    failures.push(format!(
                        "{mode}/{name}/{command}, expected {code}: {}",
                        render(&output)
                    ));
                }
            }
        }

        for name in [
            "a5_shared_view_read",
            "a5_mut_view_mutate",
            "a5_multiple_shared_views",
            "a5_slice_copy_keeps_backing_owner",
            "a5_projected_view_read",
            "a5_zero_length_slice",
        ] {
            let input = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (exe, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            if !check.status.success() || !build.status.success() {
                failures.push(format!(
                    "{mode}/{name}: check {} build {}",
                    render(&check),
                    render(&build)
                ));
                continue;
            }
            let run = Command::new(exe).output().unwrap();
            if run.status.code() != Some(0) {
                failures.push(format!("{mode}/{name} run: {}", render(&run)));
            }
        }

        let input = project.join("a5_slice_index_oob_trap.ln");
        fs::copy(fixtures.join("a5_slice_index_oob_trap.ln"), &input).unwrap();
        let (exe, build) = modes.build(root, &input, &format!("a5_slice_index_oob_trap_{mode}"));
        if !build.status.success() {
            failures.push(format!("{mode}/a5_slice_index_oob_trap build: {}", render(&build)));
        } else {
            let run = Command::new(exe).output().unwrap();
            if run.status.code() == Some(0) {
                failures.push(format!("{mode}/a5_slice_index_oob_trap did not trap: {}", render(&run)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
