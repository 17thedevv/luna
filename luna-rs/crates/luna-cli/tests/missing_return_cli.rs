//! E3004 must reject reachable fallthrough before publishing native output.
#[path = "support/stdlib.rs"]
mod support;

use std::{fs, process::Command};
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

#[test]
fn missing_return_rejects_reachable_paths_and_preserves_valid_returns_in_both_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/missing_return");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("missing_return");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build").arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(provider.with_extension("llib"))
                .env("LUNA_SYSROOT", root).output().unwrap();
            assert!(output.status.success(), "{}", render(&output));
            fs::remove_file(&provider).unwrap();
        }
        for name in ["empty", "one_branch", "loop_exit", "generic", "method", "closure", "async", "default_entry", "imported_generic"] {
            let source = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(!executable.exists(), "invalid function published output: {mode}/{name}");
            let mut messages = Vec::new();
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(1), "{mode}/{command}/{name}: {}", render(&output));
                assert!(stderr.contains("error[E3004]") && stderr.contains("-->") && stderr.contains('^'), "missing E3004 source anchor: {}", render(&output));
                assert!(!stderr.contains("error[E6001]"), "missing-return reached backend: {}", render(&output));
                messages.push(diagnostic_messages(&output));
                eprintln!("PASS {mode}/{command}/{name}: spanned E3004");
            }
            assert_eq!(messages[0], messages[1], "{mode}/{name}");
        }
        let controls = project.join("controls.ln");
        fs::copy(fixtures.join("controls.ln"), &controls).unwrap();
        let check = modes.check(root, &controls);
        assert!(check.status.success(), "{}", render(&check));
        let (executable, build) = modes.build(root, &controls, &format!("{mode}_controls"));
        assert!(build.status.success(), "{}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/controls: check, build, native exit0");
    }
}
