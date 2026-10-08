//! D1 freeze — DIAG-1..10 diagnostic conformance (contract: docs/diagnostics/diagnostics-v1.md).
//!
//! Identity is code + phase + span/origin; rendered wording is presentation only.
//! DIAG-10 registry parity is asserted by `luna-common` unit tests; DIAG-3/4/8
//! reuse the ambiguity, poison and stale-artifact reducers.
#[path = "support/stdlib.rs"]
mod support;

use std::process::Command;
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

fn count_errors(stderr: &str) -> usize {
    stderr
        .lines()
        .filter(|line| line.starts_with("error[") || line.starts_with("error:"))
        .count()
}

fn has_bare_error(stderr: &str) -> bool {
    stderr.lines().any(|line| line.starts_with("error:"))
}

#[test]
fn diagnostics_are_coded_spanned_and_phase_consistent() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/diagnostic_conformance");
    // (fixture, expected code, expected source line, phase range)
    let matrix = [
        ("parser_missing_semi", "E0001", 1u32, 1u32),
        ("unresolved_symbol", "E1001", 1, 2),
        ("use_after_move", "E3001", 5, 4),
    ];
    let mut failures = Vec::new();
    for (name, code, line, phase) in matrix {
        let fixture = fixtures.join(format!("{name}.ln"));
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            let stderr = String::from_utf8_lossy(&check.stderr).to_string();
            if check.status.code() != Some(1) {
                failures.push(format!("{mode}/{name}: expected failure: {}", render(&check)));
                continue;
            }
            if !stderr.contains(&format!("error[{code}]")) {
                failures.push(format!("{mode}/{name}: missing {code}: {}", render(&check)));
            }
            let number: u32 = code[1..].parse().unwrap();
            let actual_phase = match number / 1000 {
                0 => 1,
                1 => 2,
                3 => 4,
                _ => 0,
            };
            if actual_phase != phase {
                failures.push(format!("{mode}/{name}: {code} is not a phase-{phase} code"));
            }
            // DIAG-1: no un-coded user-visible error.
            if has_bare_error(&stderr) {
                failures.push(format!("{mode}/{name}: un-coded error line: {}", render(&check)));
            }
            // DIAG-2: primary source span with line/column.
            if !stderr
                .lines()
                .any(|l| l.trim_start().starts_with("-->") && l.contains(&format!(":{line}:")))
            {
                failures.push(format!("{mode}/{name}: missing span on line {line}: {}", render(&check)));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn diagnostics_are_deterministic_and_deduplicated() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/diagnostic_conformance");
    let mut failures = Vec::new();
    for name in ["parser_missing_semi", "unresolved_symbol", "use_after_move"] {
        let fixture = fixtures.join(format!("{name}.ln"));
        let first = modes.check(&modes.source, &fixture);
        let second = modes.check(&modes.source, &fixture);
        if first.stderr != second.stderr {
            failures.push(format!("{name}: non-deterministic diagnostics"));
        }
        let (_, build) = modes.build(&modes.source, &fixture, &format!("dedup_{name}"));
        if diagnostic_messages(&first) != diagnostic_messages(&build) {
            failures.push(format!(
                "{name}: check/build disagree: {} vs {}",
                render(&first),
                render(&build)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn related_spans_and_poison_containment_hold() {
    let modes = ProviderModes::fresh();
    let language = workspace_root().parent().unwrap().join("tests/luna/language");
    let mut failures = Vec::new();

    // DIAG-3: an ambiguous method keeps structural related locations.
    let ambiguous = language.join("impl_header_metadata/ambiguous.ln");
    let check = modes.check(&modes.source, &ambiguous);
    let stderr = String::from_utf8_lossy(&check.stderr);
    if !stderr.contains("error[E1008]") {
        failures.push(format!("DIAG-3: missing E1008: {}", render(&check)));
    }
    let arrows = stderr.lines().filter(|l| l.trim_start().starts_with("-->")).count();
    if arrows < 2 {
        failures.push(format!("DIAG-3: expected related spans, got {arrows}: {}", render(&check)));
    }

    // DIAG-4: a poisoned cast produces exactly one root diagnostic.
    let poison = language.join("cast_contract/poison_many_uses_invalid.ln");
    let check = modes.check(&modes.source, &poison);
    let errors = count_errors(&String::from_utf8_lossy(&check.stderr));
    if errors != 1 {
        failures.push(format!("DIAG-4: expected 1 diagnostic, got {errors}: {}", render(&check)));
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn origin_traceability_and_source_artifact_identity() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/diagnostic_conformance");
    let mut failures = Vec::new();

    // DIAG-9: the same consumer-side violation reports the same identity from a
    // source provider and from a relocated .llib provider.
    let mut identities = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("parity");
        let _ = std::fs::remove_dir_all(&project);
        std::fs::create_dir_all(&project).unwrap();
        let provider = project.join("provider.ln");
        std::fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
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
            std::fs::remove_file(&provider).unwrap();
        }
        let consumer = project.join("consumer.ln");
        std::fs::copy(fixtures.join("consumer.ln"), &consumer).unwrap();
        let check = modes.check(root, &consumer);
        if check.status.code() != Some(1) {
            failures.push(format!("DIAG-9 {mode}: expected failure: {}", render(&check)));
        }
        identities.push(diagnostic_messages(&check));
    }
    if identities.len() == 2 && identities[0] != identities[1] {
        failures.push(format!("DIAG-9: source/artifact identity differs: {identities:?}"));
    }

    // DIAG-8: a stale artifact diagnostic names the offending provider.
    let project = modes.artifact.join("origin");
    std::fs::create_dir_all(&project).unwrap();
    let provider = project.join("provider.ln");
    std::fs::write(&provider, "module provider { export fn value() -> i32 { return 1; } }\n").unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(project.join("provider.llib"))
        .arg("-I")
        .arg(&modes.artifact)
        .env("LUNA_SYSROOT", &modes.artifact)
        .output()
        .unwrap();
    assert!(build.status.success(), "origin provider: {}", render(&build));
    std::fs::remove_file(&provider).unwrap();
    let mut bytes = std::fs::read(project.join("provider.llib")).unwrap();
    // Corrupt the compiler version field: the diagnostic must name the provider.
    bytes[6..8].copy_from_slice(&21u16.to_le_bytes());
    std::fs::write(project.join("provider.llib"), &bytes).unwrap();
    let consumer = project.join("consumer.ln");
    std::fs::write(&consumer, "import \"provider\";\nfn main() -> i32 { return provider::value() - 1; }\n").unwrap();
    let (_, build) = modes.build(&modes.artifact, &consumer, "origin_consumer");
    let stderr = String::from_utf8_lossy(&build.stderr);
    if build.status.success() || !stderr.contains("'provider'") || !stderr.contains("compiler") {
        failures.push(format!("DIAG-8: origin/classification missing: {}", render(&build)));
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
