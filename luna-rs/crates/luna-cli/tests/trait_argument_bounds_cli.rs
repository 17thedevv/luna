#[path = "support/stdlib.rs"]
mod support;
use std::process::Command;
use support::{render, workspace_root, ProviderModes};

#[test]
fn trait_arguments_must_match_inferred_callback_types() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/trait_bounds");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for name in [
            "argument_match",
            "multiple_impl_arguments",
            "candidate_inference_rollback",
            "reference_mutability_match",
        ] {
            let valid = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &valid);
            assert!(check.status.success(), "{mode} valid: {}", render(&check));
            let (exe, build) = modes.build(root, &valid, &format!("{name}_{mode}"));
            assert!(
                build.status.success(),
                "{mode} valid build: {}",
                render(&build)
            );
            let run = Command::new(exe).output().unwrap();
            assert!(run.status.success(), "{mode} valid run: {}", render(&run));
        }

        for name in [
            "argument_mismatch",
            "reference_does_not_inherit",
            "reference_mutability_mismatch",
        ] {
            let invalid = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &invalid);
            let (_, build) = modes.build(root, &invalid, &format!("{name}_{mode}"));
            for (command, result) in [("check", check), ("build", build)] {
                assert!(
                    !result.status.success(),
                    "{mode} {command} accepted invalid bound"
                );
                assert!(
                    String::from_utf8_lossy(&result.stderr).contains("error[E2021]"),
                    "{mode} {command} wrong rejection: {}",
                    render(&result)
                );
            }
        }
    }
}
