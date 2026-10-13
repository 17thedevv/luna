//! Approved terminal-slot amendment through ordinary public Luna APIs.
#[path = "support/stdlib.rs"]
mod support;
use std::{fs, path::{Path, PathBuf}, process::{Command, Output}};
use support::{render, workspace_root, ProviderModes};

fn fixtures() -> PathBuf {
    workspace_root().parent().unwrap().join("tests/luna/stdlib/range_inclusive")
}
fn build(cli: &Path, root: &Path, source: &Path, executable: &Path) -> Output {
    Command::new(cli).arg("build").arg(source).args(["--quiet", "-o"])
        .arg(executable).env("LUNA_SYSROOT", root).output().unwrap()
}
fn publish(root: &Path, source: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build").arg(source).args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(source.with_extension("llib")).env("LUNA_SYSROOT", root).output().unwrap();
    assert!(output.status.success(), "{}", render(&output));
    fs::remove_file(source).unwrap();
}

#[test]
fn terminal_slot_preserves_checked_steps_noncopy_drops_and_source_artifact_parity() {
    let modes = ProviderModes::fresh();
    let work = luna_test_support::TempWorkspace::new("range_terminal");
    let language = workspace_root().parent().unwrap().join("tests/luna/language");
    for root in [&modes.source, &modes.artifact] {
        let project = root.join("enum_alignment");
        fs::create_dir(&project).unwrap();
        for name in ["enum_alignment_provider.ln", "enum_alignment_consumer.ln"] {
            fs::copy(language.join(name), project.join(name)).unwrap();
        }
        if root == &modes.artifact {
            publish(root, &project.join("enum_alignment_provider.ln"));
            fs::rename(project, root.join("relocated_enum_alignment")).unwrap();
        }
    }
    let mut compilers = vec![("debug", PathBuf::from(env!("CARGO_BIN_EXE_luna")))];
    if let Some(release) = std::env::var_os("LUNA_E1_RELEASE_CLI") {
        compilers.push(("release", release.into()));
    }
    let mut inputs: Vec<_> = fs::read_dir(fixtures()).unwrap().map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "ln")).collect();
    inputs.sort();
    assert_eq!(inputs.len(), 14);
    for (profile, cli) in compilers {
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            for source in &inputs {
                let name = source.file_stem().unwrap().to_str().unwrap();
                let executable = work.join(format!("{profile}_{mode}_{name}.exe"));
                let output = build(&cli, root, source, &executable);
                if name == "old_field_type_rejected" {
                    let check = modes.check(root, source);
                    for output in [&check, &output] {
                        assert_eq!(output.status.code(), Some(1), "{}", render(output));
                        assert!(String::from_utf8_lossy(&output.stderr).contains("error[E2001]"), "{}", render(output));
                    }
                    assert!(!executable.exists());
                } else {
                    assert!(output.status.success(), "{profile}/{mode}/{name}: {}", render(&output));
                    let run = Command::new(&executable).current_dir(work.as_ref()).output().unwrap();
                    if name.starts_with("trap_step_") {
                        assert!(!run.status.success() && String::from_utf8_lossy(&run.stderr)
                            .contains("luna: PANIC [L005] integer overflow"), "{profile}/{mode}/{name}: {}", render(&run));
                    } else {
                        assert_eq!(run.status.code(), Some(0), "{profile}/{mode}/{name}: {}", render(&run));
                    }
                }
            }
            eprintln!("PASS RangeInclusive {profile}/{mode}: 14 fixtures; ten checked Step traps, boundaries, exact non-Copy drops, old-field rejection");
            let project = if mode == "source" { root.join("enum_alignment") } else { root.join("relocated_enum_alignment") };
            for source in [
                project.join("enum_alignment_consumer.ln"),
                language.join("enum_payload_alignment_repro.ln"),
                language.join("enum_payload_reference_alignment_repro.ln"),
            ] {
                let name = source.file_stem().unwrap().to_str().unwrap();
                let executable = work.join(format!("{profile}_{mode}_{name}.exe"));
                let output = build(&cli, root, &source, &executable);
                assert!(output.status.success(), "{profile}/{mode}/{name}: {}", render(&output));
                let run = Command::new(executable).current_dir(work.as_ref()).output().unwrap();
                assert_eq!(run.status.code(), Some(0), "{profile}/{mode}/{name}: {}", render(&run));
            }
            eprintln!("PASS generic enum alignment {profile}/{mode}: three fixtures; constants, nested arrays/tuples, runtime/comptime size/align, aligned Option/Result projections and relocated provider");
        }
    }

    // Produce an authentic old-layout sysroot, then its dependent, using the
    // unmodified pre-amendment provider source. Never fabricate artifact bytes.
    let old = ProviderModes::fresh_with_source_edit(|providers| {
        fs::copy(fixtures().join("iter_adapters_before_option.ln.snapshot"),
            providers.join("core/iter_adapters.ln")).unwrap();
    });
    let project = old.artifact.join("range_client");
    fs::create_dir(&project).unwrap();
    let wrapper = "import <iter_adapters>; export module client { export fn make() -> std::iter::RangeInclusive<i32> { return std::iter::range_inclusive(1i32, 2i32); } }";
    fs::write(project.join("wrapper.ln"), wrapper).unwrap();
    publish(&old.artifact, &project.join("wrapper.ln"));
    fs::write(project.join("main.ln"), "import \"wrapper\"; fn main() -> i32 { dec rw values = client::make(); if values.next().unwrap() != 1i32 { return 1; } if values.next().unwrap() != 2i32 { return 2; } if values.next().is_some() { return 3; } return 0; }").unwrap();
    let relocated = old.artifact.join("relocated_range_client");
    fs::rename(project, &relocated).unwrap();
    let main = relocated.join("main.ln");
    let (exe, output) = old.build(&old.artifact, &main, "range_old_baseline");
    assert!(output.status.success(), "{}", render(&output));
    assert_eq!(Command::new(exe).output().unwrap().status.code(), Some(0));
    for extension in ["llib", "obj"] {
        fs::copy(modes.artifact.join(format!("libs/external/core/iter_adapters.{extension}")),
            old.artifact.join(format!("libs/external/core/iter_adapters.{extension}"))).unwrap();
    }
    let check = old.check(&old.artifact, &main);
    let (exe, output) = old.build(&old.artifact, &main, "range_stale");
    for output in [&check, &output] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{}", render(output));
        assert!(stderr.contains("error[E6001]") && stderr.contains("interface"), "{}", render(output));
    }
    assert!(!exe.exists());
    assert!(!relocated.join("wrapper.ln").exists());
    fs::write(relocated.join("wrapper.ln"), wrapper).unwrap();
    publish(&old.artifact, &relocated.join("wrapper.ln"));
    let (exe, output) = old.build(&old.artifact, &main, "range_rebuilt");
    assert!(output.status.success(), "{}", render(&output));
    assert_eq!(Command::new(exe).output().unwrap().status.code(), Some(0));
    eprintln!("PASS RangeInclusive old-layout dependent: baseline native, relocated stale typed check/build rejection, rebuilt native");
}
