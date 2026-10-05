#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn raw_slices_and_borrowed_views_preserve_provenance_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna");
    let language = fixtures.join("language/raw_slice_contract");
    let strings = fixtures.join("stdlib/string_views");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(language.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(project.join("provider.llib"))
                .arg("-I")
                .arg(root)
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", render(&output));
            fs::remove_file(provider).unwrap();
        }
        for (folder, name) in [
            (&language, "view_native"),
            (&language, "view_enum_native"),
            (&language, "view_callback_native"),
            (&language, "callback_scalar_native"),
            (&language, "unknown_origin_local"),
            (&language, "unknown_origin_callback_read"),
            (&strings, "native"),
        ] {
            let input = project.join(format!("{name}.ln"));
            fs::copy(folder.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (exe, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            if !check.status.success() || !build.status.success() {
                failures.push(format!(
                    "{mode}/{name}: check {} build {}",
                    render(&check),
                    render(&build)
                ));
            } else {
                let run = Command::new(exe).output().unwrap();
                if !run.status.success() {
                    failures.push(format!("{mode}/{name}: {}", render(&run)));
                } else {
                    eprintln!("PASS {mode}/{name}: check/build/native");
                }
            }
        }
        for (folder, name, code) in [
            (&language, "view_direct_conflict", "E3003"),
            (&language, "view_generic_conflict", "E3003"),
            (&language, "view_enum_conflict", "E3003"),
            (&language, "view_callback_conflict", "E3003"),
            (&language, "view_mixed_sources_conflict", "E3003"),
            (&language, "slice_middle_conflict", "E3003"),
            (&language, "slice_offset_extent_conflict", "E3003"),
            (&language, "unknown_origin_conflict", "E3003"),
            (&language, "unknown_origin_helper_conflict", "E3003"),
            (&language, "unknown_origin_ffi_conflict", "E3003"),
            (&language, "unknown_origin_callback_conflict", "E3003"),
            (&language, "unknown_origin_callback_argument_conflict", "E3003"),
            (&language, "unknown_origin_escape", "E3005"),
            (&strings, "raw_constructor_requires_unsafe", "E2025"),
            (&strings, "raw_method_requires_unsafe", "E2025"),
            (&strings, "bytes_conflict", "E3003"),
            (&strings, "bytes_escape", "E3005"),
            (&strings, "owner_move_conflict", "E3003"),
        ] {
            let input = project.join(format!("{name}.ln"));
            fs::copy(folder.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (_, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            for (command, output) in [("check", check), ("build", build)] {
                if output.status.success()
                    || !String::from_utf8_lossy(&output.stderr).contains(&format!("error[{code}]"))
                {
                    failures.push(format!(
                        "{mode}/{name}/{command}, expected {code}: {}",
                        render(&output)
                    ));
                } else {
                    eprintln!("PASS {mode}/{name}/{command}: {code}");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
