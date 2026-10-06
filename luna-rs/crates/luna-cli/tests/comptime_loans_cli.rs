#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn comptime_loan_admission_precedes_execution_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/comptime_loans");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna")).arg("build").arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"]).arg(provider.with_extension("llib"))
                .env("LUNA_SYSROOT", root).output().unwrap();
            assert!(output.status.success(), "{}", render(&output));
            fs::remove_file(provider).unwrap();
        }
        let relocated = root.join("relocated");
        fs::rename(project, &relocated).unwrap();
        for name in ["aliased_mutable", "native_aliased_mutable", "imported_aliased_mutable", "shared_mutable", "returned_loan_conflict"] {
            let source = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(!executable.exists(), "invalid comptime loan published executable");
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(1), "{}", render(&output));
                assert!(stderr.contains("error[E3003]") && stderr.contains("borrow"), "{}", render(&output));
                assert!(stderr.contains(&format!("{name}.ln:")) && stderr.contains('^'), "primary span missing: {}", render(&output));
                assert!(!stderr.contains("error[E4005]") && !stderr.contains("error[E6001]"), "loan diagnostic reclassified: {}", render(&output));
                eprintln!("PASS {mode}/{command}/{name}: E3003 before execution");
            }
        }
        let source = relocated.join("controls.ln");
        fs::copy(fixtures.join("controls.ln"), &source).unwrap();
        let check = modes.check(root, &source);
        assert!(check.status.success(), "{}", render(&check));
        let (executable, build) = modes.build(root, &source, &format!("{mode}_controls"));
        assert!(build.status.success(), "{}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/controls: sequential mutation, shared reads, precise return loan; native exit0");
    }
}
