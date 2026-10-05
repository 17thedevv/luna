#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{ProviderModes, render, workspace_root};

#[test]
fn module_constants_preserve_storage_across_native_portable_and_relocated_providers() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/module_const_storage");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        for provider in ["provider", "provider_other", "private_provider"] {
            let source = project.join(format!("{provider}.ln"));
            fs::copy(fixtures.join(format!("{provider}.ln")), &source).unwrap();
            if mode == "artifact" {
                let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                    .arg("build")
                    .arg(&source)
                    .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                    .arg(project.join(format!("{provider}.llib")))
                    .env("LUNA_SYSROOT", root)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{mode}/{provider}: {}",
                    render(&output)
                );
                fs::remove_file(&source).unwrap();
            }
        }
        // A graph whose original path no longer exists must still materialize
        // private module data in both native and portable generic bodies.
        let relocated = root.join("relocated");
        fs::rename(&project, &relocated).unwrap();
        for name in ["module_reference", "consumer", "comptime_read", "private_scopes"] {
            let input = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let (exe, build) = modes.build(root, &input, &format!("const_{name}_{mode}"));
            if !build.status.success() {
                failures.push(format!("{mode}/{name} build: {}", render(&build)));
                continue;
            }
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
        for (name, code) in [
            ("local_escape", "E3005"),
            ("local_aggregate_escape", "E3005"),
            ("mutable_module", "E2023"),
            ("private_access", "E1001"),
            ("comptime_pointer_escape", "E4005"),
        ] {
            let input = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (_, build) = modes.build(root, &input, &format!("const_{name}_{mode}"));
            for (command, output) in [("check", check), ("build", build)] {
                if output.status.success()
                    || !String::from_utf8_lossy(&output.stderr).contains(&format!("error[{code}]"))
                {
                    failures.push(format!(
                        "{mode}/{name} {command}, expected {code}: {}",
                        render(&output)
                    ));
                } else {
                    eprintln!("PASS {mode}/{name} {command}: {code}");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
