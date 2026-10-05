#[path = "support/stdlib.rs"]
mod support;

use std::{fs, process::Command};
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

#[test]
fn named_arguments_bind_declaration_ordinals_and_preserve_source_evaluation_in_both_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/named_arguments");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
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
        let relocated = root.join("relocated");
        fs::rename(project, &relocated).unwrap();
        for (name, code, message) in [
            ("unknown", "E2001", "Unknown argument label"),
            ("duplicate", "E2001", "more than one argument"),
            ("positional_duplicate", "E2001", "more than one argument"),
            ("positional_after_named", "E2001", "must precede"),
            ("missing", "E2001", "Callable expects"),
            ("opaque", "E2001", "structural callable types"),
            ("known_immutable", "E2001", "structural callable types"),
            ("impl_label", "E2001", "Unknown argument label"),
            ("unsafe_call", "E2025", "unsafe"),
            ("use_after_move", "E3001", "moved"),
            ("later_use_after_move", "E3001", "moved"),
            ("comptime_moved", "E4005", "moved"),
            ("comptime_reference_escape", "E4005", "escape"),
            ("borrow_conflict", "E3003", ""),
        ] {
            let source = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(
                !executable.exists(),
                "Invalid named call published an executable: {mode}/{name}"
            );
            let mut messages = Vec::new();
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{mode}/{command}/{name}: {}",
                    render(&output)
                );
                assert!(
                    stderr.contains(&format!("error[{code}]")) && stderr.contains(message),
                    "{}",
                    render(&output)
                );
                assert!(
                    !stderr.contains("error[E6001]"),
                    "Invalid named call reached backend: {}",
                    render(&output)
                );
                messages.push(diagnostic_messages(&output));
                eprintln!("PASS {mode}/{command}/{name}: {code} {message}");
            }
            assert_eq!(messages[0], messages[1], "{mode}/{name}");
        }
        for name in [
            "controls",
            "owned_control",
            "owned_source_order",
            "argument_early_return",
            "comptime_control",
        ] {
            let controls = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &controls).unwrap();
            let check = modes.check(root, &controls);
            assert!(check.status.success(), "{}", render(&check));
            let (executable, build) = modes.build(root, &controls, &format!("{mode}_{name}"));
            assert!(build.status.success(), "{}", render(&build));
            let run = Command::new(executable).output().unwrap();
            assert_eq!(run.status.code(), Some(0), "{}", render(&run));
            eprintln!("PASS {mode}/{name}: check, build, native exit0");
        }
    }
    verify_parameter_interface_identity(&modes, &fixtures);
}

fn publish_signature_provider(
    root: &std::path::Path,
    project: &std::path::Path,
    fixture: &std::path::Path,
) -> (
    luna_llib::format::Manifest,
    luna_llib::metadata::SemanticMetadata,
) {
    fs::create_dir_all(project).unwrap();
    let source = project.join("provider.ln");
    fs::copy(fixture, &source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(source.with_extension("llib"))
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", render(&output));
    let bytes = fs::read(source.with_extension("llib")).unwrap();
    let (_, manifest, _, metadata) =
        luna_llib::MlibReader::read_module(&mut std::io::Cursor::new(bytes)).unwrap();
    fs::remove_file(source).unwrap();
    (manifest.unwrap(), metadata.unwrap())
}

fn verify_parameter_interface_identity(modes: &ProviderModes, fixtures: &std::path::Path) {
    let root = &modes.artifact;
    let baseline_project = root.join("identity_baseline");
    let (baseline, metadata) =
        publish_signature_provider(root, &baseline_project, &fixtures.join("provider.ln"));
    let signature = metadata.interface.exported_symbols["named"].children["subtract"]
        .callable_signature
        .as_ref()
        .unwrap();
    assert_eq!(signature.parameter_names, ["left", "right"]);
    let implementation = metadata
        .interface
        .impl_headers
        .iter()
        .find(|header| {
            header.trait_id.is_some() && header.method_contracts.contains_key("subtract")
        })
        .unwrap();
    assert_eq!(
        implementation.method_contracts["subtract"]
            .callable_signature
            .as_ref()
            .unwrap()
            .parameter_names,
        ["self", "left", "right"]
    );
    // The metadata decoder is also exercised independently of portable AST rechecking.
    let decoded = luna_driver::metadata_decoder::InterfaceDecoder::new(
        luna_driver::registry::ModuleRegistry::default().allocate_id(),
        "provider".into(),
        metadata.clone(),
        Default::default(),
        baseline.provenance.interface_fingerprint,
    )
    .decode()
    .unwrap();
    assert!(decoded
        .callable_signatures
        .values()
        .any(|signature| signature.parameter_names == ["left", "right"]));
    for (variant, equal) in [
        ("parameter_rename", false),
        ("impl_rename", true),
        ("generic_rename", true),
        ("body_edit", true),
    ] {
        let (manifest, _) = publish_signature_provider(
            root,
            &root.join(variant),
            &fixtures.join(format!("{variant}.ln")),
        );
        assert_eq!(
            manifest.provenance.interface_fingerprint == baseline.provenance.interface_fingerprint,
            equal,
            "fingerprint/{variant}"
        );
        eprintln!("PASS fingerprint/{variant}: same_interface={equal}");
    }
    let project = root.join("stale_name_dependency");
    fs::create_dir(&project).unwrap();
    fs::copy(
        baseline_project.join("provider.llib"),
        project.join("provider.llib"),
    )
    .unwrap();
    let source = project.join("dependent.ln");
    fs::copy(fixtures.join("dependent.ln"), &source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(source.with_extension("llib"))
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", render(&output));
    fs::remove_file(source).unwrap();
    let consumer = project.join("main.ln");
    fs::copy(fixtures.join("dependent_consumer.ln"), &consumer).unwrap();
    let (executable, build) = modes.build(root, &consumer, "name_dependency_fresh");
    assert!(build.status.success(), "{}", render(&build));
    assert_eq!(
        Command::new(executable).output().unwrap().status.code(),
        Some(0)
    );
    fs::copy(
        root.join("parameter_rename/provider.llib"),
        project.join("provider.llib"),
    )
    .unwrap();
    let sidecar = project.join("provider.obj");
    if sidecar.exists() {
        fs::remove_file(sidecar).unwrap();
    }
    let check = modes.check(root, &consumer);
    let (executable, build) = modes.build(root, &consumer, "name_dependency_stale");
    assert!(!executable.exists());
    for output in [check, build] {
        assert_eq!(output.status.code(), Some(1), "{}", render(&output));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("dependency interface fingerprint mismatch"),
            "{}",
            render(&output)
        );
    }
    eprintln!("PASS stale parameter-name dependency: check/build reject interface mismatch");
}
