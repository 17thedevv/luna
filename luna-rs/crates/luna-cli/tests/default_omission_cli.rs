#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn omitted_defaults_execute_in_the_logical_callee_frame_in_both_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/default_arguments");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("omission_provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(provider.with_extension("llib"))
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", render(&output));
            fs::remove_file(provider).unwrap();
        }
        let relocated = root.join("relocated");
        fs::rename(project, &relocated).unwrap();
        for name in [
            "omission_scalars",
            "omission_owned",
            "omission_identity_cast_control",
            "omission_generic_owned",
            "omission_methods",
            "omission_precondition_valid",
            "omission_layout",
            "omission_comptime",
            "omission_extern",
        ] {
            let source = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(
                check.status.success(),
                "{mode}/{name}/check: {}",
                render(&check)
            );
            assert!(
                build.status.success(),
                "{mode}/{name}/build: {}",
                render(&build)
            );
            let run = Command::new(executable).output().unwrap();
            assert_eq!(
                run.status.code(),
                Some(0),
                "{mode}/{name}: {}",
                render(&run)
            );
            eprintln!("PASS {mode}/{name}: check/build/native exit0");
        }
        for (name, code) in [
            ("omission_escape", "E3005"),
            ("omission_comptime_escape", "E3005"),
            ("omission_moved", "E3001"),
            ("omission_identity_cast_moved", "E3001"),
            ("omission_precondition_inverted", "E2016"),
            ("omission_default_precondition_inverted", "E2016"),
            ("omission_owner_mismatch", "E2021"),
            ("omission_owner_arity", "E2001"),
            ("omission_method_arity", "E2001"),
            ("omission_helper_precondition_inverted", "E2016"),
            ("omission_extern_unsafe", "E2025"),
            ("omission_extern_comptime", "E4005"),
            ("omission_layout_unresolved", "E5001"),
            ("omission_helper_method_inverted", "E2016"),
            ("omission_method_precondition_inverted", "E2016"),
        ] {
            let source = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(
                !executable.exists(),
                "{mode}/{name}: invalid call published an executable; check: {}; build: {}",
                render(&check),
                render(&build)
            );
            for output in [check, build] {
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{mode}/{name}: {}",
                    render(&output)
                );
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains(&format!("error[{code}]")),
                    "{}",
                    render(&output)
                );
            }
            eprintln!("PASS {mode}/{name}: check/build {code}");
        }
    }
}
