//! C3 freeze — associated projection / fallback / proof-depth.
//!
//! Covers associated-type projection, a chained (nested) projection, deep but
//! valid bound graphs (which must not be blocked by an internal proof-depth
//! guard), and structured rejection of cyclic projections, ambiguous projections
//! and genuinely cyclic trait bounds — never a silent "unsatisfied".
#[path = "support/stdlib.rs"]
mod support;
use std::process::Command;
use support::{render, workspace_root, ProviderModes};

#[test]
fn projection_fallback_and_proof_depth_are_structured_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/projection_fallback");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for name in [
            "assoc_projection",
            "nested_projection_concrete",
            "deep_bound_60",
            "deep_bound_70",
        ] {
            let fixture = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &fixture);
            let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            if !check.status.success() || !build.status.success() {
                failures.push(format!(
                    "{mode}/{name}: check {} build {}",
                    render(&check),
                    render(&build)
                ));
                continue;
            }
            let run = Command::new(executable).output().unwrap();
            if run.status.code() != Some(0) {
                failures.push(format!("{mode}/{name} run: {}", render(&run)));
            }
        }

        for (name, code) in [
            ("cyclic_projection", "E2008"),
            ("ambiguous_projection", "E2019"),
            ("cyclic_bound", "E2021"),
        ] {
            let fixture = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &fixture);
            let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            if executable.exists() {
                failures.push(format!("{mode}/{name} published an executable"));
            }
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
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
