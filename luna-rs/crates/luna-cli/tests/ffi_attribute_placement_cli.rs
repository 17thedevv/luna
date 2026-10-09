//! D4-FU1 — `#[link]` must not rename the compiler-owned program entry.
//!
//! `#[link]` on the entry `main` is a typed front-end rejection (E0005) in both
//! `check` and `build`, with no backend E6001/ICE and no published executable.
//! `#[link]` on non-entry functions and the normal `#[link]` + `extern fn` FFI
//! forms keep working, independent of attribute position in the file.
#[path = "support/stdlib.rs"]
mod support;

use std::process::Command;
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

#[test]
fn link_on_entry_is_rejected_and_other_placements_are_accepted() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/ffi_attributes");
    let mut failures = Vec::new();

    // Negative: `#[link]` on the entry `main` -> typed reject, no ICE, no exe.
    for name in ["invalid_link_placement", "invalid_link_main_void"] {
        let fixture = fixtures.join(format!("{name}.ln"));
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            let (exe, build) = modes.build(root, &fixture, &format!("d4fu1_{name}_{mode}"));
            if exe.exists() {
                failures.push(format!("{mode}/{name}: published an executable"));
            }
            for (command, output) in [("check", &check), ("build", &build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.code() != Some(1) {
                    failures.push(format!("{mode}/{name}/{command}: {}", render(output)));
                    continue;
                }
                if !stderr.contains("error[E0005]") {
                    failures.push(format!("{mode}/{name}/{command}: expected E0005: {}", render(output)));
                }
                if stderr.contains("E6001") || stderr.contains("invariant violated") {
                    failures.push(format!("{mode}/{name}/{command}: leaked a backend ICE: {}", render(output)));
                }
            }
            if diagnostic_messages(&check) != diagnostic_messages(&build) {
                failures.push(format!("{mode}/{name}: check/build identity differs"));
            }
        }
    }

    // Positive: `#[link]` on non-entry functions and FFI forms build and run.
    for name in [
        "link_on_plain_fn",
        "link_first",
        "comment_then_link",
        "item_then_link",
    ] {
        let fixture = fixtures.join(format!("{name}.ln"));
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            let (exe, build) = modes.build(root, &fixture, &format!("d4fu1_pos_{name}_{mode}"));
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

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
