//! D4 freeze — retained contracts.
//!
//! Re-certify the nine retained contract groups on the current candidate. This
//! harness adds reducers only where the existing suites leave a gap (FFI, dyn,
//! entry, UTF-8, lifetime); closure, async, comptime and backend/provider groups
//! are covered by their dedicated suites (see the D4 ledger evidence).
#[path = "support/stdlib.rs"]
mod support;

use std::process::Command;
use support::{render, workspace_root, ProviderModes};

#[test]
fn retained_contract_groups_hold_in_source_and_artifact_modes() {
    let modes = ProviderModes::fresh();
    let language = workspace_root().parent().unwrap().join("tests/luna/language");
    let contracts = language.join("retained_contracts");
    let mut failures = Vec::new();

    // Positive: each contract group builds and runs.
    for (fixture, name) in [
        (contracts.join("lifetime_positive.ln"), "lifetime"),
        (contracts.join("ffi_positive.ln"), "ffi"),
        (contracts.join("dyn_positive.ln"), "dyn"),
        (contracts.join("entry_argv.ln"), "entry"),
        (contracts.join("utf8_positive.ln"), "utf8"),
    ] {
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            let (exe, build) = modes.build(root, &fixture, &format!("d4_{name}_{mode}"));
            if !check.status.success() || !build.status.success() {
                failures.push(format!("{mode}/{name}: {}", render(&build)));
                continue;
            }
            let run = Command::new(exe).current_dir(std::env::temp_dir()).output().unwrap();
            if run.status.code() != Some(0) {
                failures.push(format!("{mode}/{name} run: {}", render(&run)));
            }
        }
    }

    // Negative / fail-closed: each contract group rejects with its typed code.
    for (fixture, name, code) in [
        (language.join("diagnostic_conformance/use_after_move.ln"), "lifetime", "E3001"),
        (contracts.join("ffi_non_ffi_safe.ln"), "ffi", "E2030"),
        (contracts.join("dyn_not_object_safe.ln"), "dyn", "E2010"),
        (contracts.join("entry_bad_main.ln"), "entry", "E2024"),
        (language.join("diagnostic_spans/char_invalid.ln"), "utf8", "E2026"),
    ] {
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            let (exe, build) = modes.build(root, &fixture, &format!("d4neg_{name}_{mode}"));
            if exe.exists() {
                failures.push(format!("{mode}/{name}: published an executable"));
            }
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() || !stderr.contains(&format!("error[{code}]")) {
                    failures.push(format!("{mode}/{name}/{command}: expected {code}: {}", render(&output)));
                }
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
