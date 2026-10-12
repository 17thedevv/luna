//! E1 qualification: ordinary and modular reducers through source/fresh artifact
//! providers, real compiler22/23 rejection, and relocation of actual artifact bundles.
#[path = "support/stdlib.rs"]
mod support;
use std::{fs, path::Path, process::{Command, Output}};
use support::{render, workspace_root, ProviderModes};

fn build(cli: &Path, root: &Path, fixture: &Path, output: &Path) -> Output {
    Command::new(cli).arg("build").arg(fixture).args(["--quiet", "-o"])
        .arg(output).arg("-I").arg(root).env("LUNA_SYSROOT", root)
        .current_dir(output.parent().unwrap()).output().unwrap()
}

fn arithmetic_trap(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    !output.status.success() &&
        (stderr.contains("luna: PANIC [L003] division by zero") ||
         stderr.contains("luna: PANIC [L005] integer overflow"))
}

#[test]
fn checked_arithmetic_has_source_and_fresh_artifact_parity() {
    let modes = ProviderModes::fresh();
    let work = luna_test_support::TempWorkspace::new("e1_qualification");
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/checked_arithmetic");
    let debug = std::path::PathBuf::from(env!("CARGO_BIN_EXE_luna"));
    let mut compilers = vec![("debug", debug)];
    if let Some(release) = std::env::var_os("LUNA_E1_RELEASE_CLI") {
        compilers.push(("release", release.into()));
    }
    let mut inputs: Vec<_> = fs::read_dir(fixtures).unwrap().map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "ln")).collect();
    inputs.sort();
    assert_eq!(inputs.len(), 59, "reconcile fixture matrix when adding reducers");
    for (profile, cli) in &compilers {
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            for fixture in &inputs {
                let name = fixture.file_stem().unwrap().to_str().unwrap();
                let exe = work.join(format!("{profile}_{mode}_{name}.exe"));
                let output = build(cli, root, fixture, &exe);
                if name.starts_with("comptime_") {
                    assert_eq!(output.status.code(), Some(1), "{profile}/{mode}/{name}: {}", render(&output));
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert_eq!(stderr.matches("error[E4005]").count(), 1, "{}", render(&output));
                    assert!(!exe.exists());
                    continue;
                }
                assert!(output.status.success(), "{profile}/{mode}/{name}: {}", render(&output));
                let run = Command::new(&exe).current_dir(work.as_ref()).output().unwrap();
                if name.starts_with("positive_") || name.starts_with("wrapping_") {
                    assert_eq!(run.status.code(), Some(0), "{profile}/{mode}/{name}: {}", render(&run));
                } else {
                    assert!(arithmetic_trap(&run), "{profile}/{mode}/{name}: {}", render(&run));
                }
            }
            eprintln!("PASS E1 {profile}/{mode}: 59 rows, exact arithmetic panic signatures");
        }
    }
}

#[test]
fn macro_arithmetic_provider_survives_relocation_and_rejects_old_compilers() {
    let modes = ProviderModes::fresh();
    let work = luna_test_support::TempWorkspace::new("e1_provider_qualification");
    let debug = std::path::PathBuf::from(env!("CARGO_BIN_EXE_luna"));
    let mut compilers = vec![("debug", debug)];
    if let Some(release) = std::env::var_os("LUNA_E1_RELEASE_CLI") {
        compilers.push(("release", release.into()));
    }
    // Native ordinary arithmetic in a user provider, rather than only bootstrap
    // artifacts. Produce current compiler bundles through the CLI, then remove source.
    let project = work.join("provider_project");
    fs::create_dir(&project).unwrap();
    let provider = project.join("arithmetic_provider.ln");
    fs::write(&provider, "macro arithmetic { (@space: ident, @function: ident) => { export module @space { export fn @function(a: i8, b: i8) -> i8 { return a + b; } } } } arithmetic!(math, add);").unwrap();
    for (name, body, trap) in [
        ("valid", "if math::add(40i8, 2i8) == 42 { return 0; } return 1;", false),
        ("overflow", "dec value = math::add(127i8, 1i8); return 0;", true),
    ] {
        let consumer = project.join(format!("{name}.ln"));
        fs::write(&consumer, format!("import \"arithmetic_provider\"; fn main() -> i32 {{ {body} }}")).unwrap();
        let exe = work.join(format!("source_provider_{name}.exe"));
        let output = build(&compilers[0].1, &modes.source, &consumer, &exe);
        assert!(output.status.success(), "{}", render(&output));
        let run = Command::new(&exe).current_dir(work.as_ref()).output().unwrap();
        assert_eq!(arithmetic_trap(&run), trap, "{}", render(&run));
        if !trap { assert_eq!(run.status.code(), Some(0)); }
    }
    let artifact = project.join("arithmetic_provider.llib");
    let output = Command::new(&compilers[0].1).arg("build").arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"]).arg(&artifact)
        .env("LUNA_SYSROOT", &modes.source).output().unwrap();
    assert!(output.status.success(), "{}", render(&output));
    let bytes = fs::read(&artifact).unwrap();
    assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), 24);
    fs::remove_file(&provider).unwrap();
    let relocated_project = work.join("relocated_provider_project");
    fs::rename(&project, &relocated_project).unwrap();
    let relocated_sysroot = modes.artifact.with_file_name("relocated_artifact_sysroot");
    fs::rename(&modes.artifact, &relocated_sysroot).unwrap();
    assert!(!project.exists() && !modes.artifact.exists());
    for (profile, cli) in &compilers {
        for (name, trap) in [("valid", false), ("overflow", true)] {
            let consumer = relocated_project.join(format!("{name}.ln"));
            let exe = work.join(format!("{profile}_relocated_provider_{name}.exe"));
            let output = build(cli, &relocated_sysroot, &consumer, &exe);
            assert!(output.status.success(), "{}", render(&output));
            let run = Command::new(&exe).current_dir(work.as_ref()).output().unwrap();
            assert_eq!(arithmetic_trap(&run), trap, "{}", render(&run));
            if !trap { assert_eq!(run.status.code(), Some(0)); }
        }
    }

    // No source fallback: these old providers have only genuine archived llib.
    let old_consumer = relocated_project.join("old_consumer.ln");
    fs::write(&old_consumer, "import \"old_provider\"; fn main() -> i32 { return 0; }").unwrap();
    assert!(!relocated_project.join("old_provider.ln").exists());
    for (old_version, archived) in [
        (22, "compat22/num-compiler22.llib"),
        (23, "compat23/discarded-arithmetic-compiler23.llib"),
    ] {
    let archived = workspace_root().parent().unwrap().join("docs/audits/0.1-alpha-completion-2026-10-04/evidence").join(archived);
    fs::copy(archived, relocated_project.join("old_provider.llib")).unwrap();
    for (profile, cli) in &compilers {
        let exe = work.join(format!("{profile}_old_artifact.exe"));
        let output = build(cli, &relocated_sysroot, &old_consumer, &exe);
        assert_eq!(output.status.code(), Some(1), "{}", render(&output));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("compiler") && stderr.contains(&old_version.to_string()) && stderr.contains("24"), "{}", render(&output));
        assert!(!exe.exists());
    }
    }
}
