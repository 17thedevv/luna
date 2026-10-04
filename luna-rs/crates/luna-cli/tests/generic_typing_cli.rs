#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn generic_binders_stay_rigid_in_bodies_and_independent_across_relocated_providers() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/generic_typing");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let source = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &source).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&source)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(project.join("provider.llib"))
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{mode}/provider: {}",
                render(&output)
            );
            fs::remove_file(&source).unwrap();
        }
        let relocated = root.join("relocated");
        fs::rename(&project, &relocated).unwrap();
        for name in ["consumer", "nested_struct_literal_required_accept"] {
            let input = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let (exe, build) = modes.build(root, &input, &format!("generic_{name}_{mode}"));
            if !build.status.success() {
                failures.push(format!("{mode}/{name} build: {}", render(&build)));
            } else {
                let run = Command::new(exe)
                    .current_dir(std::env::temp_dir())
                    .output()
                    .unwrap();
                if !run.status.success() {
                    failures.push(format!("{mode}/{name} native: {}", render(&run)));
                } else {
                    eprintln!("PASS {mode}/{name}: relocated graph, native exit0");
                }
            }
        }
        for name in [
            "rigid_return_required_reject",
            "literal_return_reject",
            "unrelated_binder_reject",
            "local_initializer_reject",
            "explicit_argument_reject",
            "struct_initializer_reject",
        ] {
            let input = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (exe, build) = modes.build(root, &input, &format!("generic_{name}_{mode}"));
            assert!(
                !exe.exists(),
                "rejected generic body produced executable: {exe:?}"
            );
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success()
                    || !stderr.contains("error[E2001]")
                    || stderr.contains("error[E5001]")
                {
                    failures.push(format!(
                        "{mode}/{name} {command}, expected E2001: {}",
                        render(&output)
                    ));
                } else {
                    eprintln!("PASS {mode}/{name} {command}: E2001");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
