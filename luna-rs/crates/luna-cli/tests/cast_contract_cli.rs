#[path = "support/stdlib.rs"]
mod stdlib;

use std::process::Command;
use stdlib::{render, workspace_root, ProviderModes};

#[test]
fn casts_preserve_type_capability_and_unsafe_admission_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/cast_contract");
    for (name, code) in [
        ("nominal_privacy", "E2026"),
        ("shared_to_mutable", "E2026"),
        ("reference_reinterpret", "E2026"),
        ("unsafe_callable_erasure", "E2026"),
        ("generic_nominal_to_integer", "E2026"),
        ("integer_to_rigid_generic", "E2026"),
        ("raw_reference_without_unsafe", "E2025"),
        ("private_direct_control", "E1003"),
        ("identity_move_reject", "E3001"),
    ] {
        let fixture = fixtures.join(format!("{name}.ln"));
        let mut reasons = Vec::new();
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            for command in ["check", "build"] {
                let output = if command == "check" {
                    modes.check(root, &fixture)
                } else {
                    let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
                    assert!(!executable.exists());
                    build
                };
                assert_eq!(output.status.code(), Some(1), "{mode}/{command}/{name}: {}", render(&output));
                assert!(String::from_utf8_lossy(&output.stderr).contains(code), "{}", render(&output));
                reasons.push(stdlib::diagnostic_messages(&output));
            }
        }
        assert!(reasons.windows(2).all(|p| p[0] == p[1]), "{name}: {reasons:?}");
    }
    for name in ["controls", "raw_callable_abi_control", "identity_drop"] {
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let fixture = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &fixture);
            assert!(check.status.success(), "{}", render(&check));
            let (executable, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            assert!(build.status.success(), "{}", render(&build));
            let run = Command::new(executable).output().unwrap();
            assert_eq!(run.status.code(), Some(0), "{mode}/{name}: {}", render(&run));
        }
    }
}
