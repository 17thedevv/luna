#[path = "support/stdlib.rs"]
mod stdlib;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use stdlib::{ProviderModes, render, workspace_root};

fn check(root: &Path, fixture: &Path, search: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("check")
        .arg(fixture)
        .arg("--quiet")
        .arg("-I")
        .arg(search)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap()
}

fn rejects(output: &Output, code: &str) -> String {
    assert!(!output.status.success(), "must reject: {}", render(output));
    let text = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(
        text.contains(&format!("error[{code}]")),
        "{}",
        render(output)
    );
    assert!(
        !text.lines().any(|line| line.starts_with("error:")),
        "uncoded error: {text}"
    );
    text
}

#[test]
fn provider_authority_and_diagnostics_have_source_artifact_parity() {
    let modes = ProviderModes::fresh();
    let cases = modes.source.join("integrity_cases");
    fs::create_dir_all(&cases).unwrap();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/compiler_integrity");
    let rogue = cases.join("rogue_search");
    fs::create_dir_all(rogue.join("core")).unwrap();
    fs::copy(fixtures.join("rogue_slice.ln"), rogue.join("core/slice.ln")).unwrap();
    let binary_copy = cases.join("binary_copy");
    fs::create_dir_all(binary_copy.join("core")).unwrap();
    fs::copy(
        modes.artifact.join("libs/external/core/slice.llib"),
        binary_copy.join("core/slice.llib"),
    )
    .unwrap();
    // The untrusted search directories have neither a manifest nor source fallback
    // for the relocated artifact. Artifact identity alone is not authorization.
    assert!(!binary_copy.join("core/slice.ln").exists());
    for root in [&modes.source, &modes.artifact] {
        rejects(
            &check(root, &fixtures.join("slice_consumer.ln"), &rogue),
            "E2006",
        );
        rejects(
            &check(root, &fixtures.join("rogue_slice.ln"), root),
            "E2006",
        );
        let copied = check(root, &fixtures.join("slice_consumer.ln"), &binary_copy);
        if root == &modes.artifact {
            // With fresh artifact dependencies, reach the actual authorization gate.
            rejects(&copied, "E2006");
        } else {
            // Mixing an artifact with source dependencies fails fingerprint validation
            // first. Preserve strict rejection rather than bypassing that validator.
            let text = rejects(&copied, "E6001");
            assert!(
                text.contains("dependency interface fingerprint mismatch"),
                "{text}"
            );
        }

        // Explicitly searching the actual canonical directory must retain authority.
        assert!(
            check(
                root,
                &fixtures.join("slice_consumer.ln"),
                &root.join("libs/external")
            )
            .status
            .success()
        );
        let (executable, build) = modes.build(
            root,
            &fixtures.join("slice_iteration.ln"),
            if root == &modes.source {
                "slice_source"
            } else {
                "slice_artifact"
            },
        );
        assert!(build.status.success(), "{}", render(&build));
        assert_eq!(
            Command::new(executable).output().unwrap().status.code(),
            Some(0)
        );

        let text = rejects(
            &check(root, &fixtures.join("uninitialized.ln"), root),
            "E3006",
        );
        assert!(text.contains(":3:12"), "{text}");
        let text = rejects(
            &check(root, &fixtures.join("unicode_missing_field.ln"), root),
            "E1001",
        );
        let source = fs::read_to_string(fixtures.join("unicode_missing_field.ln")).unwrap();
        let line = source.lines().nth(1).unwrap();
        let prefix = &line[..line.find("nope").unwrap()];
        let column = prefix.chars().count() + 1;
        assert!(text.contains(&format!(":2:{column}\n")), "{text}");
        assert!(
            text.contains(&format!("   | {}^^^^\n", " ".repeat(column - 1))),
            "{text}"
        );

        // Ordinary nominal inherent impls from a search path remain available.
        let nominal_search = cases.join("nominal_search");
        fs::create_dir_all(nominal_search.join("core")).unwrap();
        fs::copy(
            fixtures.join("nominal_provider.ln"),
            nominal_search.join("core/slice.ln"),
        )
        .unwrap();
        assert!(
            check(root, &fixtures.join("nominal_consumer.ln"), &nominal_search)
                .status
                .success()
        );
    }
}
