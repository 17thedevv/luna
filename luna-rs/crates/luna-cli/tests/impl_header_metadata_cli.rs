//! C2 freeze — per-impl-header metadata.
//!
//! Proves each checked impl header keeps its identity, binders, constraints and
//! method contracts through `.llib` serialization, decoding and reconstruction:
//! impls on different concrete self types or different trait arguments are never
//! merged, generic-impl constraints survive, the graph still resolves from a
//! relocated artifact, and an ambiguous method call is rejected rather than
//! resolved to the first candidate.
#[path = "support/stdlib.rs"]
mod support;

use luna_llib::{metadata::SemanticMetadata, MlibReader};
use std::{fs, path::Path, process::Command};
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

fn publish(root: &Path, directory: &Path, fixture: &Path) -> SemanticMetadata {
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
    assert!(output.status.success(), "{}", render(&output));
    let bytes = fs::read(&artifact).unwrap();
    let (_, _, _, metadata) =
        MlibReader::read_module(&mut std::io::Cursor::new(bytes)).unwrap();
    fs::remove_file(&source).unwrap();
    metadata.unwrap()
}

#[test]
fn per_impl_header_metadata_round_trips_and_resolves_deterministically() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/impl_header_metadata");
    let metadata = publish(
        &modes.artifact,
        &modes.artifact.join("provider"),
        &fixtures.join("provider.ln"),
    );

    let describes: Vec<_> = metadata
        .interface
        .impl_headers
        .iter()
        .filter(|header| header.method_contracts.contains_key("describe"))
        .collect();
    assert_eq!(describes.len(), 2, "two concrete Describe impls must survive");
    assert_ne!(describes[0].self_type, describes[1].self_type);
    assert_ne!(describes[0].identity, describes[1].identity);
    assert_ne!(
        describes[0].method_contracts["describe"].symbol_id,
        describes[1].method_contracts["describe"].symbol_id
    );

    let ranks: Vec<_> = metadata
        .interface
        .impl_headers
        .iter()
        .filter(|header| header.method_contracts.contains_key("rank"))
        .collect();
    assert_eq!(ranks.len(), 2, "two Ranked impls must survive");
    assert_ne!(
        ranks[0].trait_args.first(),
        ranks[1].trait_args.first(),
        "same nominal head with different trait arguments must not be merged"
    );
    assert_ne!(ranks[0].identity, ranks[1].identity);

    let generic = metadata
        .interface
        .impl_headers
        .iter()
        .find(|header| header.method_contracts.contains_key("inner"))
        .expect("generic impl header");
    assert_eq!(
        generic.constraints.traits.len(),
        1,
        "generic impl-level bound must survive"
    );

    let mut failures = Vec::new();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("consumer");
        let _ = fs::remove_dir_all(&project);
        fs::create_dir_all(&project).unwrap();
        if mode == "source" {
            fs::copy(fixtures.join("provider.ln"), project.join("provider.ln")).unwrap();
        } else {
            fs::copy(
                root.join("provider/provider.llib"),
                project.join("provider.llib"),
            )
            .unwrap();
            let sidecar = root.join("provider/provider.obj");
            if sidecar.exists() {
                fs::copy(sidecar, project.join("provider.obj")).unwrap();
            }
        }
        let relocated = root.join("relocated");
        let _ = fs::remove_dir_all(&relocated);
        fs::rename(&project, &relocated).unwrap();
        fs::copy(fixtures.join("consumer.ln"), relocated.join("consumer.ln")).unwrap();
        let (executable, build) = modes.build(
            root,
            &relocated.join("consumer.ln"),
            &format!("impl_consumer_{mode}"),
        );
        if !build.status.success() {
            failures.push(format!("{mode}/consumer build: {}", render(&build)));
        } else {
            let run = Command::new(executable)
                .current_dir(std::env::temp_dir())
                .output()
                .unwrap();
            if run.status.code() != Some(0) {
                failures.push(format!("{mode}/consumer run: {}", render(&run)));
            }
        }

        let input = root.join("ambiguous.ln");
        fs::copy(fixtures.join("ambiguous.ln"), &input).unwrap();
        let check = modes.check(root, &input);
        let (executable, build) = modes.build(root, &input, &format!("impl_ambiguous_{mode}"));
        if executable.exists() {
            failures.push(format!("{mode}/ambiguous published an executable"));
        }
        if check.status.success() || build.status.success() {
            failures.push(format!(
                "{mode}/ambiguous accepted: {}",
                render(&check)
            ));
        } else if !String::from_utf8_lossy(&check.stderr).contains("error[E1008]") {
            failures.push(format!("{mode}/ambiguous code: {}", render(&check)));
        } else if diagnostic_messages(&check) != diagnostic_messages(&build) {
            failures.push(format!("{mode}/ambiguous unstable: {}", render(&check)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
