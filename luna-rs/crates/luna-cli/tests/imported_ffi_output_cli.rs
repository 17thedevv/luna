#[path = "support/stdlib.rs"]
mod support;

use std::{fs, process::Command};
use support::{ProviderModes, render, workspace_root};

#[test]
fn imported_extern_outputs_preserve_ffi_identity_and_borrow_checks() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/imported_ffi_output");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        for name in ["consumer", "reject_borrow", "reject_unsafe"] {
            fs::copy(
                fixtures.join(format!("{name}.ln")),
                project.join(format!("{name}.ln")),
            )
            .unwrap();
        }
        if mode == "source" {
            fs::copy(fixtures.join("bridge.ln"), project.join("bridge.ln")).unwrap();
        } else {
            let built = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(fixtures.join("bridge.ln"))
                .args(["--emit", "llib", "--lib", "--quiet", "-o"])
                .arg(project.join("bridge.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(built.status.success(), "{}", render(&built));
            assert!(!project.join("bridge.ln").exists());
        }
        let source = project.join("consumer.ln");
        let check = modes.check(root, &source);
        assert!(check.status.success(), "{mode}: {}", render(&check));
        let (exe, build) = modes.build(root, &source, &format!("ffi_{mode}"));
        assert!(build.status.success(), "{mode}: {}", render(&build));
        assert!(Command::new(exe).status().unwrap().success());
        for (name, code) in [
            ("reject_borrow", "error[E3003]"),
            ("reject_unsafe", "error[E2025]"),
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
                    String::from_utf8_lossy(&output.stderr).contains(code),
                    "{mode}/{name}: {}",
                    render(&output)
                );
            }
        }
    }
}
