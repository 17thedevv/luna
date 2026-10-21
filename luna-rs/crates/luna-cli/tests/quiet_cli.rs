use std::{path::Path, process::{Command, Output}};

fn invoke(command: &str, fixture: &Path, quiet: bool, output: &Path) -> Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_luna"));
    process.arg(command).arg(fixture).arg("--no-config");
    if quiet { process.arg("--quiet"); }
    if command == "build" { process.arg("-o").arg(output); }
    process.current_dir(output.parent().unwrap()).output().unwrap()
}

#[test]
fn quiet_commands_have_clean_streams_and_keep_typed_failures() {
    let work = luna_test_support::TempWorkspace::new("quiet_cli");
    let good = work.join("main.ln");
    std::fs::write(&good, "fn main() -> i32 { return 0; }\n").unwrap();
    let bad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
        .join("tests/luna/language/retained_contracts/entry_bad_main.ln");
    let output = work.join("app").with_extension(std::env::consts::EXE_EXTENSION);
    for command in ["check", "build", "run"] {
        let result = invoke(command, &good, true, &output);
        assert!(result.status.success(), "{command}: {}", String::from_utf8_lossy(&result.stderr));
        assert!(result.stdout.is_empty(), "{command}: unexpected stdout: {}", String::from_utf8_lossy(&result.stdout));
        assert!(result.stderr.is_empty(), "{command}: unexpected stderr: {}", String::from_utf8_lossy(&result.stderr));
        let failure = invoke(command, &bad, true, &output);
        assert!(!failure.status.success(), "{command}: invalid entry accepted");
        let error = String::from_utf8_lossy(&failure.stderr);
        assert!(error.contains("error[E2024]"), "{command}: missing original typed diagnostic: {error}");
        assert!(!error.contains("DEBUG"), "{command}: importer noise: {error}");
    }
    let normal = invoke("check", &good, false, &output);
    assert!(normal.status.success());
    assert!(normal.stderr.is_empty());
    assert!(String::from_utf8_lossy(&normal.stdout).contains("Check successfully"));
}
