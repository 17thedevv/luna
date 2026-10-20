//! D1-FU2 freeze — comptime diagnostic cascade.
//!
//! A semantic error diagnosed inside a comptime target must not be restated as
//! a generic comptime failure (E4005). An independent comptime evaluation
//! failure must still report E4005. Identity is code + phase; the interpreter
//! boundary preserves provenance via `ComptimeError::AlreadyDiagnosed` instead
//! of stringifying the generator's diagnostics.
#[path = "support/stdlib.rs"]
mod support;

use support::{render, workspace_root, ProviderModes};

fn codes(stderr: &str) -> Vec<String> {
    let mut found: Vec<String> = stderr
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("error[") {
                rest.split(']').next().map(|code| code.to_string())
            } else if line.starts_with("error:") {
                Some("BARE".to_string())
            } else {
                None
            }
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

fn fixture(name: &str) -> std::path::PathBuf {
    workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/comptime_diagnostic_cascade")
        .join(format!("{name}.ln"))
}

#[test]
fn semantic_error_inside_comptime_is_not_restated() {
    let modes = ProviderModes::fresh();
    let path = fixture("semantic_inside_comptime");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let check = modes.check(root, &path);
        let stderr = String::from_utf8_lossy(&check.stderr).to_string();
        let found = codes(&stderr);
        if check.status.code() != Some(1) {
            failures.push(format!("{mode}: expected failure: {}", render(&check)));
            continue;
        }
        if !found.iter().any(|c| c == "E1001") {
            failures.push(format!("{mode}: missing root E1001: {}", render(&check)));
        }
        if found.iter().any(|c| c == "E4005") {
            failures.push(format!("{mode}: redundant E4005 cascade: {}", render(&check)));
        }
        if found.iter().any(|c| c == "BARE") {
            failures.push(format!("{mode}: un-coded error line: {}", render(&check)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn independent_comptime_failure_still_reports_e4005() {
    let modes = ProviderModes::fresh();
    let path = fixture("independent_comptime_failure");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let check = modes.check(root, &path);
        let stderr = String::from_utf8_lossy(&check.stderr).to_string();
        let found = codes(&stderr);
        if !found.iter().any(|c| c == "E4005") {
            failures.push(format!("{mode}: independent failure lost E4005: {}", render(&check)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn semantic_error_outside_comptime_is_unchanged() {
    let modes = ProviderModes::fresh();
    let path = fixture("semantic_outside_comptime");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let check = modes.check(root, &path);
        let stderr = String::from_utf8_lossy(&check.stderr).to_string();
        let found = codes(&stderr);
        if !found.iter().any(|c| c == "E1001") {
            failures.push(format!("{mode}: missing E1001: {}", render(&check)));
        }
        if found.iter().any(|c| c == "E4005") {
            failures.push(format!("{mode}: unexpected E4005: {}", render(&check)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn check_and_build_agree_on_cascade_identity() {
    let modes = ProviderModes::fresh();
    let path = fixture("semantic_inside_comptime");
    let check = modes.check(&modes.source, &path);
    let (_, built) = modes.build(&modes.source, &path, "d1fu2_cascade");
    let check_codes = codes(&String::from_utf8_lossy(&check.stderr));
    let build_codes = codes(&String::from_utf8_lossy(&built.stderr));
    assert_eq!(
        check_codes, build_codes,
        "check/build diagnostic identity mismatch:\ncheck: {}\nbuild: {}",
        render(&check),
        render(&built)
    );
    assert!(!check_codes.iter().any(|c| c == "E4005"), "cascade not suppressed: {:?}", check_codes);
}
