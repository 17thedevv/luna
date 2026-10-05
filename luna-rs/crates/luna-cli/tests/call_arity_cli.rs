#[path = "support/stdlib.rs"]
mod support;

use std::{fs, process::Command};
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

#[test]
fn call_arity_rejects_at_semantic_boundary_and_preserves_valid_calls_in_both_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/call_arity");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("arity_provider.ln");
        fs::copy(fixtures.join("arity_provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build").arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(provider.with_extension("llib"))
                .env("LUNA_SYSROOT", root).output().unwrap();
            assert!(output.status.success(), "{}", render(&output));
            fs::remove_file(provider).unwrap();
        }
        let relocated = root.join("relocated");
        fs::rename(project, &relocated).unwrap();
        for name in [
            "direct_missing", "direct_excess", "generic_missing", "generic_excess",
            "opaque_missing", "opaque_excess", "field_missing", "field_excess",
            "closure_missing", "closure_excess", "qualified_missing", "qualified_excess",
            "variadic_missing_fixed",
        ] {
            let source = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(!executable.exists(), "Invalid arity published an executable: {mode}/{name}");
            let mut messages = Vec::new();
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(1), "{mode}/{command}/{name}: {}", render(&output));
                assert!(stderr.contains("error[E2001]") && stderr.contains("Callable expects"), "{}", render(&output));
                assert!(!stderr.contains("error[E6001]"), "Invalid arity reached backend: {}", render(&output));
                messages.push(diagnostic_messages(&output));
                eprintln!("PASS {mode}/{command}/{name}: E2001 callable arity");
            }
            assert_eq!(messages[0], messages[1], "{mode}/{name}");
        }
        let controls = relocated.join("controls.ln");
        fs::copy(fixtures.join("controls.ln"), &controls).unwrap();
        let check = modes.check(root, &controls);
        assert!(check.status.success(), "{}", render(&check));
        let (executable, build) = modes.build(root, &controls, &format!("{mode}_controls"));
        assert!(build.status.success(), "{}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/controls: check, build, native exit0");

        // Preserve the existing semantic rule for an explicitly variadic
        // declaration. This is a check-only control, not native variadic ABI
        // certification; structural function pointers have no such declaration.
        let source = relocated.join("variadic_semantic_control.ln");
        fs::copy(fixtures.join("variadic_semantic_control.ln"), &source).unwrap();
        let check = modes.check(root, &source);
        assert!(check.status.success(), "{}", render(&check));
        eprintln!("PASS {mode}/variadic_semantic_control: check only");
    }
}
