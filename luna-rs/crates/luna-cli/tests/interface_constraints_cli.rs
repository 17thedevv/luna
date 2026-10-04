#[path = "support/stdlib.rs"]
mod support;

use luna_llib::{metadata::SemanticMetadata, MlibReader};
use std::{fs, path::Path, process::Command};
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

fn publish(
    root: &Path,
    directory: &Path,
    fixture: &Path,
) -> (luna_llib::format::Manifest, SemanticMetadata) {
    fs::create_dir_all(directory).unwrap();
    let source = directory.join("provider.ln");
    let artifact = source.with_extension("llib");
    fs::copy(fixture, &source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(&artifact)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}: {}",
        fixture.display(),
        render(&output)
    );
    let bytes = fs::read(artifact).unwrap();
    let (_, manifest, _, metadata) =
        MlibReader::read_module(&mut std::io::Cursor::new(bytes)).unwrap();
    fs::remove_file(source).unwrap();
    (manifest.unwrap(), metadata.unwrap())
}

#[test]
fn public_constraints_change_identity_without_binder_spelling_or_declaration_order_leaking() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/interface_constraints");
    let (baseline, metadata) = publish(
        &modes.artifact,
        &modes.artifact.join("baseline"),
        &fixtures.join("baseline.ln"),
    );
    assert_eq!(
        metadata.interface.impl_headers.len(),
        10,
        "each checked public impl must survive independently"
    );
    assert_eq!(
        metadata.interface.traits.len(),
        6,
        "public trait definitions must be populated"
    );
    let root = &metadata.interface.exported_symbols["api"];
    let identity = &root.children["identity"];
    assert_eq!(identity.constraints.traits.len(), 1);
    assert_eq!(
        identity.constraints.traits[0].param,
        identity.generic_params[0]
    );
    let generic = metadata
        .interface
        .impl_headers
        .iter()
        .find(|header| header.method_contracts.contains_key("echo"))
        .unwrap();
    assert_eq!(generic.constraints.traits.len(), 1);
    let echo = &generic.method_contracts["echo"];
    assert_eq!(echo.constraints.traits.len(), 1);
    assert_ne!(generic.generic_params[0], echo.generic_params[0]);
    assert!(metadata
        .interface
        .impl_headers
        .iter()
        .any(|header| !header.constraints.associated_equalities.is_empty()));
    assert!(metadata
        .interface
        .impl_headers
        .iter()
        .any(|header| !header.associated_types.is_empty()));
    assert_eq!(root.children["Bounded"].constraints.traits.len(), 1);
    assert_eq!(root.children["BoundedTrait"].constraints.traits.len(), 1);
    assert_eq!(
        root.children["HasBoundMethod"].children["accept"]
            .constraints
            .traits
            .len(),
        1
    );
    let read_headers: Vec<_> = metadata
        .interface
        .impl_headers
        .iter()
        .filter(|header| header.methods.contains_key("read"))
        .collect();
    assert_eq!(
        read_headers.len(),
        2,
        "two concrete self types must not be merged by nominal head"
    );
    assert_ne!(read_headers[0].self_type, read_headers[1].self_type);
    assert_ne!(read_headers[0].identity, read_headers[1].identity);
    // The metadata decoder is an internal invariant, distinct from canonical
    // artifact execution through the required portable AST below.
    let decoded = luna_driver::metadata_decoder::InterfaceDecoder::new(
        luna_driver::registry::ModuleRegistry::default().allocate_id(),
        "provider".into(),
        metadata.clone(),
        Default::default(),
        baseline.provenance.interface_fingerprint,
    )
    .decode()
    .unwrap();
    assert_eq!(decoded.checked_impl_headers.len(), 10);
    assert_eq!(decoded.trait_bounds.len(), 7);
    assert_eq!(decoded.assoc_type_bounds.len(), 1);
    assert_eq!(decoded.decl_associated_types.len(), 1);
    assert!(decoded
        .trait_associated_types
        .values()
        .any(|types| types.len() == 1));
    assert_eq!(decoded.raw_generic_param_symbols.len(), 9);

    for variant in [
        "impl_bound",
        "method_bound",
        "function_bound",
        "nominal_bound",
        "trait_bound",
        "trait_method_bound",
        "associated_equality",
        "associated_definition",
        "private_body",
        "alpha_renamed",
        "reversed",
    ] {
        let (manifest, _) = publish(
            &modes.artifact,
            &modes.artifact.join(variant),
            &fixtures.join(format!("{variant}.ln")),
        );
        let equal = ["private_body", "alpha_renamed", "reversed"].contains(&variant);
        assert_eq!(manifest.provenance.interface_fingerprint == baseline.provenance.interface_fingerprint, equal,
            "{variant}: public contract identity must reflect semantics, not presentation or private bodies");
        assert_ne!(
            manifest.provenance.source_fingerprint,
            baseline.provenance.source_fingerprint
        );
        eprintln!(
            "PASS fingerprint/{variant}: same_interface={equal}, baseline={}, variant={}",
            baseline.provenance.interface_fingerprint, manifest.provenance.interface_fingerprint
        );
    }
    // Corruption controls use a genuine public artifact rather than a synthetic
    // shape which merely duplicates the implementation's construction rules.
    let mut invalid = metadata.interface.clone();
    invalid
        .exported_symbols
        .get_mut("api")
        .unwrap()
        .children
        .get_mut("identity")
        .unwrap()
        .constraints
        .traits[0]
        .param = generic.generic_params[0].clone();
    assert!(
        luna_llib::reader::validate_generic_contracts(&invalid).is_err(),
        "cross-owner bound must reject"
    );
    let mut invalid = metadata.interface.clone();
    invalid.impl_headers[0].self_type = u32::MAX;
    assert!(
        luna_llib::reader::validate_generic_contracts(&invalid).is_err(),
        "invalid self type index must reject"
    );

    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for variant in ["baseline", "alpha_renamed", "reversed"] {
            let project = root.join(format!("consumer_{variant}"));
            fs::create_dir(&project).unwrap();
            if mode == "source" {
                fs::copy(
                    fixtures.join(format!("{variant}.ln")),
                    project.join("provider.ln"),
                )
                .unwrap();
            } else {
                fs::copy(
                    root.join(variant).join("provider.llib"),
                    project.join("provider.llib"),
                )
                .unwrap();
            }
            let relocated = root.join(format!("relocated_{variant}"));
            fs::rename(project, &relocated).unwrap();
            fs::copy(fixtures.join("consumer.ln"), relocated.join("consumer.ln")).unwrap();
            let (exe, build) = modes.build(
                root,
                &relocated.join("consumer.ln"),
                &format!("interface_{mode}_{variant}"),
            );
            assert!(
                build.status.success(),
                "{mode}/{variant}: {}",
                render(&build)
            );
            let run = Command::new(exe)
                .current_dir(std::env::temp_dir())
                .output()
                .unwrap();
            assert!(run.status.success(), "{mode}/{variant}: {}", render(&run));
            eprintln!("PASS native/{mode}/{variant}: relocated graph, exit0");
            for negative in [
                "missing_impl_bound",
                "missing_method_bound",
                "missing_function_bound",
                "missing_associated_equality",
                "missing_nominal_bound",
                "missing_inferred_nominal_bound",
                "unknown_bound",
                "non_trait_bound",
                "unknown_associated_bound",
            ] {
                let input = relocated.join(format!("{negative}.ln"));
                fs::copy(fixtures.join(format!("{negative}.ln")), &input).unwrap();
                let check = modes.check(root, &input);
                let (exe, build) =
                    modes.build(root, &input, &format!("{negative}_{mode}_{variant}"));
                assert!(
                    !check.status.success() && !build.status.success() && !exe.exists(),
                    "{mode}/{variant}/{negative}: check={} build={}",
                    render(&check),
                    render(&build)
                );
                assert_eq!(diagnostic_messages(&check), diagnostic_messages(&build));
                assert!(!String::from_utf8_lossy(&build.stderr).contains("error[E5001]"));
                eprintln!("PASS reject/{mode}/{variant}/{negative}: check/build agree");
            }
        }
    }
    let project = modes.artifact.join("stale_dependency");
    fs::create_dir(&project).unwrap();
    fs::copy(
        modes.artifact.join("baseline/provider.llib"),
        project.join("provider.llib"),
    )
    .unwrap();
    let source = project.join("dependent.ln");
    fs::copy(fixtures.join("dependent.ln"), &source).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(source.with_extension("llib"))
        .env("LUNA_SYSROOT", &modes.artifact)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "dependent publish: {}",
        render(&build)
    );
    fs::remove_file(source).unwrap();
    let consumer = project.join("main.ln");
    fs::copy(fixtures.join("dependent_consumer.ln"), &consumer).unwrap();
    let (exe, build) = modes.build(&modes.artifact, &consumer, "fresh_constraint_dependency");
    assert!(
        build.status.success(),
        "fresh dependency: {}",
        render(&build)
    );
    assert!(Command::new(exe).output().unwrap().status.success());
    for variant in [
        "impl_bound",
        "method_bound",
        "function_bound",
        "nominal_bound",
        "trait_bound",
        "trait_method_bound",
        "associated_equality",
        "associated_definition",
    ] {
        fs::copy(
            modes.artifact.join(variant).join("provider.llib"),
            project.join("provider.llib"),
        )
        .unwrap();
        let check = modes.check(&modes.artifact, &consumer);
        let (exe, build) = modes.build(
            &modes.artifact,
            &consumer,
            &format!("stale_constraint_{variant}"),
        );
        assert!(!exe.exists());
        for output in [check, build] {
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                !output.status.success()
                    && error.contains("dependency interface fingerprint mismatch"),
                "stale/{variant}: {}",
                render(&output)
            );
        }
        eprintln!("PASS stale/{variant}: check/build reject dependency interface mismatch");
    }
}
