#[path = "support/stdlib.rs"]
mod support;
use std::{fs, path::Path, process::Command};
use support::{render, workspace_root, ProviderModes};

fn rename_memory_declarations(root: &Path) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rename_memory_declarations(&path);
        } else if path.extension().is_some_and(|e| e == "ln") {
            let source = fs::read_to_string(&path).unwrap();
            let renamed = source
                .replace("fn drop_in_place<", "fn destroy_payload<")
                .replace("::drop_in_place", "::destroy_payload")
                .replace("fn slice_from_raw_parts_mut<", "fn reconstruct_mutable<")
                .replace("::slice_from_raw_parts_mut", "::reconstruct_mutable")
                .replace("fn slice_from_raw_parts<", "fn reconstruct_readonly<")
                .replace("::slice_from_raw_parts", "::reconstruct_readonly");
            if source != renamed {
                fs::write(path, renamed).unwrap();
            }
        }
    }
}

fn verify_callable_metadata(root: &Path, project: &Path) {
    let input = project.join("signatures.ln");
    let artifact = project.join("signatures.llib");
    let mut fingerprints = Vec::new();
    for unsafe_callback in [false, true] {
        let qualifier = if unsafe_callback { "unsafe " } else { "" };
        fs::write(&input, format!(
            "export module signatures {{ export fn accept(value: {qualifier}fn(i32) -> i32) -> i32 {{ return 0; }} }}"
        )).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_luna"))
            .arg("build")
            .arg(&input)
            .args(["--lib", "--emit", "llib", "--quiet", "-o"])
            .arg(&artifact)
            .arg("-I")
            .arg(root)
            .env("LUNA_SYSROOT", root)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", render(&output));
        let bytes = fs::read(&artifact).unwrap();
        let (_, manifest, _, metadata) =
            luna_llib::MlibReader::read_module(&mut std::io::Cursor::new(bytes)).unwrap();
        let metadata = metadata.unwrap();
        let types = &metadata.interface.types;
        assert!(types.iter().any(|ty| match ty {
            luna_llib::metadata::CanonicalType::Function { params, is_unsafe: false, .. } if params.len() == 1 => {
                matches!(&types[params[0] as usize], luna_llib::metadata::CanonicalType::Function { is_unsafe, .. }
                    if *is_unsafe == unsafe_callback)
            }
            _ => false,
        }), "serialized callback signature lost its callable safety");
        let fingerprint = manifest.unwrap().provenance.interface_fingerprint;
        assert_eq!(fingerprint, metadata.interface.fingerprint().unwrap());
        fingerprints.push(fingerprint);
    }
    assert_ne!(
        fingerprints[0], fingerprints[1],
        "public callable safety must change interface identity"
    );
}

fn exercise(modes: &ProviderModes, renamed: bool) {
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/intrinsic_identity");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            verify_callable_metadata(root, &project);
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
        for name in ["ordinary_calls", "ordinary_owned_pointer", "genuine_memory"] {
            let input = project.join(format!("{name}.ln"));
            let mut source = fs::read_to_string(fixtures.join(format!("{name}.ln"))).unwrap();
            if renamed && name == "genuine_memory" {
                source = source
                    .replace("::drop_in_place", "::destroy_payload")
                    .replace("::slice_from_raw_parts_mut", "::reconstruct_mutable")
                    .replace("::slice_from_raw_parts", "::reconstruct_readonly");
            }
            fs::write(&input, source).unwrap();
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
                    eprintln!("PASS renamed={renamed} {mode}/{name}: check/build/native exit0");
                }
            }
        }
        for (name, code) in [
            ("spoof_hook", "E0005"),
            ("missing_intrinsic_argument", "E2001"),
            ("excess_intrinsic_argument", "E2001"),
            ("unsafe_intrinsic_call", "E2025"),
            ("direct_local_escape", "E3005"),
            ("indirect_local_escape", "E3005"),
            ("indirect_unsafe_call", "E2025"),
            ("unsafe_user_callback", "E2025"),
            ("unsafe_returned_callback", "E2025"),
            ("unsafe_generic_callback", "E2025"),
            ("unsafe_callback_erasure", "E2001"),
        ] {
            let input = project.join(format!("{name}.ln"));
            let mut source = fs::read_to_string(fixtures.join(format!("{name}.ln"))).unwrap();
            if renamed {
                source = source
                    .replace("::drop_in_place", "::destroy_payload")
                    .replace("::slice_from_raw_parts", "::reconstruct_readonly");
            }
            fs::write(&input, source).unwrap();
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
                    eprintln!("PASS renamed={renamed} {mode}/{name}/{command}: {code}");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn user_names_remain_ordinary_and_explicit_memory_hooks_keep_native_behavior() {
    exercise(&ProviderModes::fresh(), false);
}

#[test]
fn renamed_trusted_memory_hooks_keep_source_artifact_and_function_pointer_behavior() {
    exercise(
        &ProviderModes::fresh_with_source_edit(rename_memory_declarations),
        true,
    );
}

// This independent reference-loan regression remains a release blocker.
#[test]
fn raw_slice_reference_conflicts_are_rejected_in_source_and_artifact_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/intrinsic_identity");
    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for name in ["direct_borrow_conflict", "indirect_borrow_conflict"] {
            let input = root.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &input).unwrap();
            let check = modes.check(root, &input);
            let (_, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            for (command, output) in [("check", check), ("build", build)] {
                if output.status.success()
                    || !String::from_utf8_lossy(&output.stderr).contains("error[E3003]")
                {
                    failures.push(format!(
                        "{mode}/{name}/{command}, expected E3003: {}",
                        render(&output)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
