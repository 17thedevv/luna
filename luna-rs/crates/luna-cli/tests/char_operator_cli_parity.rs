#[path = "support/stdlib.rs"]
mod stdlib;
use std::fs;
use std::process::Command;
use stdlib::{render, workspace_root, ProviderModes};

#[test]
fn char_operators_reject_before_codegen_in_source_and_fresh_artifact_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().join("tests/luna/compiler");
    let mut names = Vec::new();
    for operator in [
        "add",
        "sub",
        "mul",
        "div",
        "mod",
        "bit_and",
        "bit_or",
        "bit_xor",
        "shift_left",
        "shift_right",
    ] {
        names.push(format!("char_operator_{operator}"));
        names.push(format!("char_assignment_{operator}"));
    }
    names.extend(
        [
            "char_operator_neg",
            "char_operator_bit_not",
            "char_operator_alias",
            "char_operator_field",
            "char_operator_const",
            "char_operator_generic",
            "char_assignment_generic",
            "char_neg_generic",
        ]
        .map(str::to_owned),
    );
    for name in names {
        let fixture = fixtures.join(format!("{name}.ln"));
        let mut diagnostics = Vec::new();
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            for command in ["check", "build"] {
                let output = if command == "check" {
                    modes.check(root, &fixture)
                } else {
                    let (executable, output) =
                        modes.build(root, &fixture, &format!("{mode}_{name}"));
                    assert!(!executable.exists());
                    output
                };
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{mode}/{name}/{command}: {}",
                    render(&output)
                );
                let message = std::str::from_utf8(&output.stderr).unwrap();
                assert!(
                    message.contains("E_INVALID_CHAR_OPERATOR"),
                    "{mode}/{name}: {message}"
                );
                assert!(!message.contains("panicked"), "{message}");
                diagnostics.push(stdlib::diagnostic_messages(&output));
            }
        }
        assert!(
            diagnostics.windows(2).all(|pair| pair[0] == pair[1]),
            "diagnostic parity for {name}: {diagnostics:?}"
        );
    }
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let fixture = fixtures.join("char_operator_controls.ln");
        let (executable, build) = modes.build(root, &fixture, &format!("{mode}_controls"));
        assert!(build.status.success(), "{}", render(&build));
        let output = Command::new(executable).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{mode}: {}", render(&output));
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
}

#[test]
fn imported_generic_operators_are_rechecked_after_artifact_instantiation() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().join("tests/luna/compiler");
    let provider = fixtures.join("providers/char_operators.ln");
    // Rebuild the ordinary user provider, not a compiler-recognized stdlib type.
    let source_dir = modes.source.join("user");
    let artifact_dir = modes.artifact.join("user");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&artifact_dir).unwrap();
    fs::copy(&provider, source_dir.join("char_operators.ln")).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet"])
        .arg("-o")
        .arg(artifact_dir.join("char_operators.llib"))
        .arg("-I")
        .arg(&modes.artifact)
        .env("LUNA_SYSROOT", &modes.artifact)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", render(&build));
    assert!(artifact_dir.join("char_operators.llib").exists());
    assert!(!artifact_dir.join("char_operators.ln").exists());

    for name in ["add", "shift", "negate", "increment", "controls"] {
        let mut reasons = Vec::new();
        for (mode, root, directory) in [
            ("source", &modes.source, &source_dir),
            ("artifact", &modes.artifact, &artifact_dir),
        ] {
            let fixture = directory.join(format!("char_provider_{name}.ln"));
            fs::copy(fixtures.join(format!("char_provider_{name}.ln")), &fixture).unwrap();
            let (executable, build) =
                modes.build(root, &fixture, &format!("{mode}_provider_{name}"));
            if name == "controls" {
                assert!(build.status.success(), "{mode}: {}", render(&build));
                let run = Command::new(executable).output().unwrap();
                assert_eq!(run.status.code(), Some(0), "{mode}: {}", render(&run));
                assert!(run.stdout.is_empty() && run.stderr.is_empty());
            } else {
                assert_eq!(
                    build.status.code(),
                    Some(1),
                    "{mode}/{name}: {}",
                    render(&build)
                );
                assert!(!executable.exists());
                assert!(
                    String::from_utf8_lossy(&build.stderr).contains("E_INVALID_CHAR_OPERATOR"),
                    "{}",
                    render(&build)
                );
                reasons.push(stdlib::diagnostic_messages(&build));
                let check = modes.check(root, &fixture);
                assert_eq!(check.status.code(), Some(1), "{}", render(&check));
                reasons.push(stdlib::diagnostic_messages(&check));
            }
        }
        assert!(
            reasons.windows(2).all(|pair| pair[0] == pair[1]),
            "{name}: {reasons:?}"
        );
    }
}
