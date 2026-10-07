//! A4 freeze — cast / poison containment.
//!
//! Invalid static casts must fail in the semantic layer with one typed
//! diagnostic, must poison the operand (no diagnostic cascade), must not reach
//! MVIR/backend, and must not publish an executable. Runtime-trapping casts
//! (integer-to-char) must stay well-typed and trap deterministically.
#[path = "support/stdlib.rs"]
mod stdlib;

use std::{fs, process::Command};
use stdlib::{render, workspace_root, ProviderModes};

fn count_errors(stderr: &str) -> usize {
    stderr
        .lines()
        .filter(|line| line.starts_with("error[") || line.starts_with("error:"))
        .count()
}

#[test]
fn invalid_static_casts_are_contained_in_all_positions() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/cast_contract");
    let mut failures = Vec::new();
    for name in [
        "nested_cast_invalid",
        "cast_in_argument_invalid",
        "cast_in_return_invalid",
        "cast_in_condition_invalid",
        "cast_in_comptime_invalid",
        "poison_many_uses_invalid",
    ] {
        let fixture = fixtures.join(format!("{name}.ln"));
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            if check.status.code() != Some(1) {
                failures.push(format!("{mode}/{name} check: {}", render(&check)));
                continue;
            }
            let stderr = String::from_utf8_lossy(&check.stderr);
            if !stderr.contains("E2026") {
                failures.push(format!("{mode}/{name} missing E2026: {}", render(&check)));
            }
            let errors = count_errors(&stderr);
            if errors != 1 {
                failures.push(format!("{mode}/{name} cascade ({errors} errors): {}", render(&check)));
            }
            let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            if build.status.code() != Some(1) || executable.exists() {
                failures.push(format!("{mode}/{name} published artifact: {}", render(&build)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn invalid_static_cast_never_reaches_mvir() {
    let modes = ProviderModes::fresh();
    let fixture = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/cast_contract/poison_many_uses_invalid.ln");
    let emitted = std::env::temp_dir().join("a4_poison_invalid.mvir");
    let _ = fs::remove_file(&emitted);
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&fixture)
        .args(["--emit", "mvir", "--quiet", "-o"])
        .arg(&emitted)
        .arg("-I")
        .arg(&modes.source)
        .env("LUNA_SYSROOT", &modes.source)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", render(&output));
    assert!(
        !emitted.exists(),
        "MVIR was emitted for an invalid static cast: {}",
        render(&output)
    );
}

#[test]
fn provider_body_invalid_cast_is_rejected_in_source_and_artifact() {
    let modes = ProviderModes::fresh();
    let dir = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/cast_contract/provider_body_invalid");
    let provider = dir.join("provider.ln");
    let consumer = dir.join("consumer.ln");
    let mut failures = Vec::new();

    let out = modes.check(&modes.source, &provider);
    if out.status.code() != Some(1) || !String::from_utf8_lossy(&out.stderr).contains("E2026") {
        failures.push(format!("source/provider: {}", render(&out)));
    }

    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::copy(&consumer, project.join("consumer.ln")).unwrap();
        if mode == "source" {
            fs::copy(&provider, project.join("provider.ln")).unwrap();
        }
        let out = modes.check(&project, &project.join("consumer.ln"));
        if out.status.code() != Some(1) {
            failures.push(format!("{mode}/consumer: {}", render(&out)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn valid_casts_survive_the_provider_artifact_round_trip() {
    let modes = ProviderModes::fresh();
    let dir = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/cast_contract/provider_valid_cast");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(dir.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let build = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(project.join("provider.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(build.status.success(), "{mode}/provider: {}", render(&build));
            fs::remove_file(&provider).unwrap();
        }
        let consumer = project.join("consumer.ln");
        fs::copy(dir.join("consumer.ln"), &consumer).unwrap();
        let (executable, build) = modes.build(root, &consumer, &format!("{mode}_valid_cast"));
        if !build.status.success() {
            failures.push(format!("{mode}/consumer build: {}", render(&build)));
            continue;
        }
        let run = Command::new(executable).output().unwrap();
        if run.status.code() != Some(0) {
            failures.push(format!("{mode}/consumer run: {}", render(&run)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn runtime_trapping_casts_stay_well_typed_and_trap_deterministically() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/cast_contract");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let valid = fixtures.join("char_cast_runtime_valid.ln");
        let (executable, build) = modes.build(root, &valid, &format!("{mode}_char_valid"));
        assert!(build.status.success(), "{mode}/valid build: {}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{mode}/valid run: {}", render(&run));

        let trap = fixtures.join("char_cast_runtime_trap.ln");
        let (executable, build) = modes.build(root, &trap, &format!("{mode}_char_trap"));
        assert!(build.status.success(), "{mode}/trap build: {}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_ne!(
            run.status.code(),
            Some(0),
            "{mode}/trap did not trap: {}",
            render(&run)
        );
    }
}
