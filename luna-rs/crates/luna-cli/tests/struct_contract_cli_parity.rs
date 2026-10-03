#[path = "support/stdlib.rs"]
mod stdlib;

use std::fs;
use std::process::Command;
use stdlib::{render, workspace_root, ProviderModes};

#[test]
fn comma_separated_struct_anchors_survive_source_and_fresh_artifact_loading() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().join("tests/luna/compiler");
    let provider = fixtures.join("providers/owner_anchor_list.ln");
    let source = modes.source.join("user");
    let artifact = modes.artifact.join("user");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&artifact).unwrap();
    fs::copy(&provider, source.join("owner_anchor_list.ln")).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet"])
        .arg("-o")
        .arg(artifact.join("owner_anchor_list.llib"))
        .arg("-I")
        .arg(&modes.artifact)
        .env("LUNA_SYSROOT", &modes.artifact)
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", render(&build));
    assert!(artifact.join("owner_anchor_list.llib").exists());
    assert!(artifact.join("owner_anchor_list.obj").exists());
    assert!(!artifact.join("owner_anchor_list.ln").exists());

    let mut executions = Vec::new();
    let mut reasons = vec![Vec::new(); 5];
    for (mode, root, directory) in [
        ("source", &modes.source, &source),
        ("artifact", &modes.artifact, &artifact),
    ] {
        let positive = directory.join("owner_anchor_list_acceptance.ln");
        fs::copy(fixtures.join("owner_anchor_list_acceptance.ln"), &positive).unwrap();
        let check = modes.check(root, &positive);
        assert!(check.status.success(), "{mode}: {}", render(&check));
        let (executable, build) = modes.build(root, &positive, &format!("{mode}_anchors"));
        assert!(build.status.success(), "{mode}: {}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{mode}: {}", render(&run));
        assert!(run.stdout.is_empty() && run.stderr.is_empty());
        executions.push((run.status.code(), run.stdout, run.stderr));

        for (index, (name, expected)) in [
            ("wrong_first_owner", "LifetimeConstraintViolation"),
            ("wrong_second_owner", "LifetimeConstraintViolation"),
            ("duplicate", "duplicate raw storage anchor"),
            ("non_pointer", "E_RAW_STORAGE_ANCHOR_FIELD"),
            (
                "missing_field",
                "raw storage anchor field 'missing' not found",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let filename = format!("owner_anchor_list_{name}.ln");
            let fixture = directory.join(&filename);
            fs::copy(fixtures.join(filename), &fixture).unwrap();
            for command in ["check", "build"] {
                let output = if command == "check" {
                    modes.check(root, &fixture)
                } else {
                    let (executable, build) =
                        modes.build(root, &fixture, &format!("{mode}_{name}"));
                    assert!(!executable.exists());
                    build
                };
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{mode}/{name}/{command}: {}",
                    render(&output)
                );
                let diagnostic = std::str::from_utf8(&output.stderr).unwrap();
                assert!(
                    diagnostic.contains(expected),
                    "expected {expected}: {diagnostic}"
                );
                assert!(!diagnostic.contains("panicked"));
                reasons[index].push(stdlib::diagnostic_messages(&output));
            }
        }
    }
    assert_eq!(executions[0], executions[1]);
    for diagnostics in reasons {
        assert!(
            diagnostics.windows(2).all(|pair| pair[0] == pair[1]),
            "{diagnostics:?}"
        );
    }
}
