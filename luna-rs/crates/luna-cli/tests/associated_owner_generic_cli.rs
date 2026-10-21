#[path = "support/stdlib.rs"]
mod support;

use std::{fs, process::Command};
use support::{ProviderModes, render, workspace_root};

#[test]
fn associated_owner_arguments_instantiate_impl_binders_and_bounds() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/associated_owner_generic");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        for name in [
            "local",
            "consumer",
            "inferred",
            "reject_bound",
            "reject_owner",
            "reject_bound_value",
            "reject_inferred_bound_value",
            "reject_later_bound_value",
            "reject_caller_bound_value",
        ] {
            fs::copy(
                fixtures.join(format!("{name}.ln")),
                project.join(format!("{name}.ln")),
            )
            .unwrap();
        }
        if mode == "source" {
            fs::copy(fixtures.join("provider.ln"), project.join("provider.ln")).unwrap();
        } else {
            let built = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(fixtures.join("provider.ln"))
                .args(["--emit", "llib", "--lib", "--quiet", "-o"])
                .arg(project.join("provider.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(built.status.success(), "{}", render(&built));
            assert!(!project.join("provider.ln").exists());
        }
        for name in ["local", "consumer", "inferred"] {
            let source = project.join(format!("{name}.ln"));
            let check = modes.check(root, &source);
            assert!(check.status.success(), "{mode}/{name}: {}", render(&check));
            let (exe, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(build.status.success(), "{mode}/{name}: {}", render(&build));
            let run = Command::new(exe).output().unwrap();
            assert!(run.status.success(), "{mode}/{name}: {}", render(&run));
        }
        for (name, message) in [
            ("reject_bound", "error[E2021]"),
            ("reject_owner", "error[E2001]"),
            ("reject_bound_value", "error[E2021]"),
            ("reject_inferred_bound_value", "error[E2021]"),
            ("reject_later_bound_value", "error[E2021]"),
            ("reject_caller_bound_value", "error[E2021]"),
        ] {
            let source = project.join(format!("{name}.ln"));
            let check = modes.check(root, &source);
            let (exe, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(!exe.exists());
            for output in [check, build] {
                assert!(
                    !output.status.success(),
                    "{mode}/{name}: {}",
                    render(&output)
                );
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains(message),
                    "{mode}/{name}: {}",
                    render(&output)
                );
            }
        }
    }
}
