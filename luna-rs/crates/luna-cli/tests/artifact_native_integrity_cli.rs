#[path = "support/artifact_payload.rs"]
mod payload;
#[path = "support/stdlib.rs"]
mod support;

use luna_llib::{Fingerprint, LlibHeader, LlibReader, SectionEntry, SectionType};
use std::{
    fs,
    io::{Cursor, Seek, SeekFrom},
    path::Path,
    process::Command,
};
use support::{render, workspace_root, ProviderModes};

fn publish(modes: &ProviderModes, fixture: &Path, folder: &Path) -> Vec<u8> {
    fs::create_dir_all(folder).unwrap();
    let source = folder.join("provider.ln");
    let artifact = source.with_extension("llib");
    fs::copy(fixture, &source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(&artifact)
        .env("LUNA_SYSROOT", &modes.artifact)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", render(&output));
    fs::remove_file(source).unwrap();
    fs::read(artifact).unwrap()
}

fn corrupt_payload(bytes: &[u8], kind: SectionType) -> Vec<u8> {
    let mut input = Cursor::new(bytes);
    let header = LlibHeader::read_from(&mut input).unwrap();
    input
        .seek(SeekFrom::Start(header.section_table_offset))
        .unwrap();
    for _ in 0..header.section_count {
        let entry = SectionEntry::read_from(&mut input).unwrap();
        if entry.section_type == kind {
            assert!(entry.size > 0);
            let mut changed = bytes.to_vec();
            changed[(entry.offset + entry.size - 1) as usize] ^= 1;
            return changed;
        }
    }
    panic!("missing payload {kind:?}");
}

fn invalid_object_section(bytes: &[u8], field: &str) -> Vec<u8> {
    let mut input = Cursor::new(bytes);
    let header = LlibHeader::read_from(&mut input).unwrap();
    input
        .seek(SeekFrom::Start(header.section_table_offset))
        .unwrap();
    for _ in 0..header.section_count {
        let row = input.position() as usize;
        let entry = SectionEntry::read_from(&mut input).unwrap();
        if entry.section_type == SectionType::ObjectCode {
            let mut changed = bytes.to_vec();
            match field {
                "missing" => changed[row + 4..row + 8]
                    .copy_from_slice(&(SectionType::Custom as u32).to_le_bytes()),
                "offset" => changed[row + 8..row + 16].copy_from_slice(&u64::MAX.to_le_bytes()),
                "size" => changed[row + 16..row + 24].copy_from_slice(&u64::MAX.to_le_bytes()),
                "overlap" => changed[row + 8..row + 16]
                    .copy_from_slice(&header.section_table_offset.to_le_bytes()),
                _ => panic!("unknown corruption"),
            }
            return changed;
        }
    }
    panic!("missing object");
}

fn corrupt_generic_literal(bytes: &[u8]) -> Vec<u8> {
    let mut input = Cursor::new(bytes);
    let header = LlibHeader::read_from(&mut input).unwrap();
    input
        .seek(SeekFrom::Start(header.section_table_offset))
        .unwrap();
    for _ in 0..header.section_count {
        let entry = SectionEntry::read_from(&mut input).unwrap();
        if entry.section_type == SectionType::AstInterface {
            let payload = &bytes[entry.offset as usize..(entry.offset + entry.size) as usize];
            let needle = [6u64.to_le_bytes().as_slice(), b"725169"].concat();
            let locations: Vec<_> = payload
                .windows(needle.len())
                .enumerate()
                .filter_map(|(index, window)| (window == needle).then_some(index))
                .collect();
            assert_eq!(locations.len(), 1);
            let start = entry.offset as usize + locations[0] + 8;
            let mut changed = bytes.to_vec();
            changed[start..start + 6].copy_from_slice(b"725171");
            assert_eq!(
                changed
                    .windows(b"return 725169;".len())
                    .filter(|window| *window == b"return 725169;")
                    .count(),
                bytes
                    .windows(b"return 725169;".len())
                    .filter(|window| *window == b"return 725169;")
                    .count()
            );
            return changed;
        }
    }
    panic!("missing AST");
}

#[test]
fn native_payload_integrity_rejects_replacements_and_invalid_envelopes_before_use() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/artifact_integrity");
    let baseline = publish(
        &modes,
        &fixtures.join("provider.ln"),
        &modes.artifact.join("publish_original"),
    );
    let changed = publish(
        &modes,
        &fixtures.join("changed.ln"),
        &modes.artifact.join("publish_changed"),
    );
    let original_object = LlibReader::read_object_code(&mut Cursor::new(&baseline))
        .unwrap()
        .unwrap();
    let changed_object = LlibReader::read_object_code(&mut Cursor::new(&changed))
        .unwrap()
        .unwrap();
    assert_ne!(original_object, changed_object);
    assert_eq!(
        original_object.len(),
        changed_object.len(),
        "replacement must reach the digest check"
    );
    let manifest = LlibReader::read_manifest(&mut Cursor::new(&baseline)).unwrap();
    let metadata = manifest.object_metadata.unwrap();
    assert_eq!(metadata.hash, Fingerprint::from_slice(&original_object).0);
    assert_eq!(metadata.size, original_object.len() as u64);
    assert_eq!(metadata.format, manifest.target.object_format);

    let project = modes.artifact.join("consumer_project");
    fs::create_dir(&project).unwrap();
    fs::copy(fixtures.join("consumer.ln"), project.join("consumer.ln")).unwrap();
    let relocated = modes.artifact.join("relocated_consumer_project");
    fs::rename(project, &relocated).unwrap();
    let destination = relocated.join("provider.llib");
    assert!(!relocated.join("provider.ln").exists());
    for (tag, artifact, exit) in [("baseline", &baseline, 0), ("fresh_changed", &changed, 6)] {
        fs::write(&destination, artifact).unwrap();
        let sidecar = destination.with_extension("obj");
        if sidecar.exists() {
            fs::remove_file(&sidecar).unwrap();
        }
        let (exe, build) = modes.build(&modes.artifact, &relocated.join("consumer.ln"), tag);
        assert!(build.status.success(), "{tag}: {}", render(&build));
        assert_eq!(
            Command::new(exe).output().unwrap().status.code(),
            Some(exit)
        );
        eprintln!("PASS native/{tag}: artifact-only relocated provider, exit{exit}");
    }
    let replaced = payload::rewrite(&baseline, |kind, data| {
        if kind == SectionType::ObjectCode {
            *data = changed_object.clone();
        }
    });
    let missing = payload::change_manifest(&baseline, |manifest| manifest.object_metadata = None);
    let wrong_size = payload::change_manifest(&baseline, |manifest| {
        manifest.object_metadata.as_mut().unwrap().size += 1
    });
    let wrong_hash = payload::change_manifest(&baseline, |manifest| {
        manifest.object_metadata.as_mut().unwrap().hash[0] ^= 1
    });
    let wrong_format = payload::change_manifest(&baseline, |manifest| {
        manifest.object_metadata.as_mut().unwrap().format = "invalid".into()
    });
    let empty = payload::rewrite(&baseline, |kind, data| {
        if kind == SectionType::ObjectCode {
            data.clear();
        }
    });
    for (case, invalid, reason) in [
        ("replacement", replaced, "object hash mismatch"),
        ("missing_metadata", missing, "missing object metadata"),
        ("wrong_size", wrong_size, "object size mismatch"),
        ("wrong_hash", wrong_hash, "object hash mismatch"),
        (
            "wrong_format",
            wrong_format,
            "object metadata format mismatch",
        ),
        ("empty", empty, "empty object payload"),
        (
            "corrupt_ast",
            corrupt_payload(&baseline, SectionType::AstInterface),
            "SectionChecksumMismatch(15)",
        ),
        (
            "corrupt_mvir",
            corrupt_payload(&baseline, SectionType::GenericMVIR),
            "SectionChecksumMismatch(7)",
        ),
        (
            "corrupt_semantic",
            corrupt_payload(&baseline, SectionType::SemanticMetadata),
            "SectionChecksumMismatch(16)",
        ),
        (
            "corrupt_manifest",
            corrupt_payload(&baseline, SectionType::Manifest),
            "SectionChecksumMismatch(10)",
        ),
        (
            "corrupt_native",
            corrupt_payload(&baseline, SectionType::ObjectCode),
            "SectionChecksumMismatch(8)",
        ),
        (
            "missing_payload",
            invalid_object_section(&baseline, "missing"),
            "object metadata without payload",
        ),
        (
            "invalid_offset",
            invalid_object_section(&baseline, "offset"),
            "CorruptedData",
        ),
        (
            "invalid_size",
            invalid_object_section(&baseline, "size"),
            "CorruptedData",
        ),
        (
            "overlap",
            invalid_object_section(&baseline, "overlap"),
            "CorruptedData",
        ),
    ] {
        assert!(
            LlibReader::read_manifest(&mut Cursor::new(&invalid)).is_err(),
            "{case}/manifest"
        );
        assert!(
            LlibReader::read_object_code(&mut Cursor::new(&invalid)).is_err(),
            "{case}/object"
        );
        assert!(
            LlibReader::read_module(&mut Cursor::new(&invalid)).is_err(),
            "{case}/module"
        );
        assert!(
            LlibReader::read_ast_interface(&mut Cursor::new(&invalid)).is_err(),
            "{case}/ast"
        );
        fs::write(&destination, &invalid).unwrap();
        for matching_sidecar in [false, true] {
            let sidecar = destination.with_extension("obj");
            if sidecar.exists() {
                fs::remove_file(&sidecar).unwrap();
            }
            if matching_sidecar {
                fs::write(
                    &sidecar,
                    if case == "replacement" {
                        &changed_object
                    } else {
                        &original_object
                    },
                )
                .unwrap();
            }
            for command in ["check", "build"] {
                let exe = relocated
                    .join(format!("rejected_{case}_{matching_sidecar}_{}", command))
                    .with_extension(std::env::consts::EXE_EXTENSION);
                let mut invocation = Command::new(env!("CARGO_BIN_EXE_luna"));
                invocation
                    .arg(command)
                    .arg(relocated.join("consumer.ln"))
                    .arg("--quiet");
                if command == "build" {
                    invocation.arg("-o").arg(&exe);
                }
                let output = invocation
                    .env("LUNA_SYSROOT", &modes.artifact)
                    .output()
                    .unwrap();
                let text = render(&output);
                assert!(
                    !output.status.success()
                        && text.contains("error[E6001]")
                        && text.contains(reason),
                    "{case}/{matching_sidecar}/{command}: {text}"
                );
                assert!(!exe.exists());
                eprintln!(
                    "PASS reject/{case}/sidecar={matching_sidecar}/{command}: E6001, {reason}"
                );
            }
        }
    }
    // A failed import must not poison later valid use.
    fs::write(&destination, baseline).unwrap();
    fs::write(destination.with_extension("obj"), original_object).unwrap();
    let (exe, build) = modes.build(&modes.artifact, &relocated.join("consumer.ln"), "restored");
    assert!(build.status.success(), "{}", render(&build));
    assert_eq!(Command::new(exe).output().unwrap().status.code(), Some(0));
    eprintln!("PASS native/restored: exit0");

    let generic = publish(
        &modes,
        &fixtures.join("generic.ln"),
        &modes.artifact.join("publish_generic"),
    );
    fs::write(&destination, &generic).unwrap();
    fs::write(
        destination.with_extension("obj"),
        LlibReader::read_object_code(&mut Cursor::new(&generic))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    fs::copy(
        fixtures.join("generic_consumer.ln"),
        relocated.join("consumer.ln"),
    )
    .unwrap();
    let source_project = modes.source.join("generic_source");
    fs::create_dir(&source_project).unwrap();
    fs::copy(
        fixtures.join("generic.ln"),
        source_project.join("provider.ln"),
    )
    .unwrap();
    fs::copy(
        fixtures.join("generic_consumer.ln"),
        source_project.join("consumer.ln"),
    )
    .unwrap();
    for (mode, root, consumer) in [
        ("source", &modes.source, source_project.join("consumer.ln")),
        ("artifact", &modes.artifact, relocated.join("consumer.ln")),
    ] {
        let (exe, build) = modes.build(root, &consumer, &format!("generic_{mode}"));
        assert!(build.status.success(), "{}", render(&build));
        assert_eq!(Command::new(exe).output().unwrap().status.code(), Some(0));
        eprintln!("PASS native/generic/{mode}: exit0");
    }
    fs::write(&destination, corrupt_generic_literal(&generic)).unwrap();
    for command in ["check", "build"] {
        let exe = relocated
            .join(format!("rejected_generic_{command}"))
            .with_extension(std::env::consts::EXE_EXTENSION);
        let mut invocation = Command::new(env!("CARGO_BIN_EXE_luna"));
        invocation
            .arg(command)
            .arg(relocated.join("consumer.ln"))
            .arg("--quiet");
        if command == "build" {
            invocation.arg("-o").arg(&exe);
        }
        let output = invocation
            .env("LUNA_SYSROOT", &modes.artifact)
            .output()
            .unwrap();
        let text = render(&output);
        assert!(
            !output.status.success()
                && text.contains("error[E6001]")
                && text.contains("SectionChecksumMismatch(15)"),
            "generic/{command}: {text}"
        );
        assert!(!exe.exists());
        eprintln!("PASS reject/generic/{command}: changed AST literal, unchanged source text/object, E6001");
    }
}

#[test]
fn artifact_publication_failure_is_visible_in_quiet_and_verbose_cli_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/artifact_integrity");
    let project = modes.artifact.join("publication_project");
    fs::create_dir(&project).unwrap();
    let source = project.join("provider.ln");
    fs::copy(fixtures.join("provider.ln"), &source).unwrap();
    for quiet in [false, true] {
        let destination = project.join(format!("blocked_{quiet}.llib"));
        fs::create_dir(&destination).unwrap();
        let marker = destination.join("preserved.txt");
        fs::write(&marker, b"keep").unwrap();
        let mut invocation = Command::new(env!("CARGO_BIN_EXE_luna"));
        invocation
            .arg("build")
            .arg(&source)
            .args(["--lib", "--emit", "llib", "-o"])
            .arg(&destination);
        if quiet {
            invocation.arg("--quiet");
        }
        let output = invocation
            .env("LUNA_SYSROOT", &modes.artifact)
            .output()
            .unwrap();
        let text = render(&output);
        assert!(
            !output.status.success()
                && text.contains("error[E6006]")
                && text.contains("Failed to publish library artifact"),
            "quiet={quiet}: {text}"
        );
        assert!(destination.is_dir());
        assert_eq!(fs::read(marker).unwrap(), b"keep");
        assert!(!fs::read_dir(&project).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp.")));
        eprintln!("PASS reject/publication/quiet={quiet}: E6006, destination preserved, temporary cleaned");
    }
    let artifact = project.join("valid.llib");
    for round in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_luna"))
            .arg("build")
            .arg(&source)
            .args(["--lib", "--emit", "llib", "--quiet", "-o"])
            .arg(&artifact)
            .env("LUNA_SYSROOT", &modes.artifact)
            .output()
            .unwrap();
        assert!(output.status.success(), "round{round}: {}", render(&output));
        assert!(LlibReader::read_module(&mut Cursor::new(fs::read(&artifact).unwrap())).is_ok());
        eprintln!("PASS publication/round{round}: readable complete artifact");
    }
}
