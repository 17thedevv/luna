//! C-GAP: generic native math FFI in source and freshly built artifact providers.
#[path = "support/stdlib.rs"]
mod support;

use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn user_math_ffi_links_and_runs_in_source_and_artifact_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/native_math_link");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("native_math_link");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
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
        let main = project.join("main.ln");
        fs::copy(fixtures.join("main.ln"), &main).unwrap();
        let check = modes.check(root, &main);
        assert!(check.status.success(), "{mode}: {}", render(&check));
        let (executable, build) = modes.build(root, &main, &format!("{mode}_math_ffi"));
        assert!(build.status.success(), "{mode}: {}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{mode}: {}", render(&run));
        eprintln!("PASS {mode}: check, build, native math FFI exit0");
    }
}
