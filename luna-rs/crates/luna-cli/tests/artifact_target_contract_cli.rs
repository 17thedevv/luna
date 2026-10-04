#[path = "support/stdlib.rs"]
mod support;

use luna_llib::{LlibHeader, LlibReader, SectionEntry, SectionType};
use std::{
    fs,
    io::{Cursor, Seek, SeekFrom},
    process::Command,
};
use support::{render, workspace_root, ProviderModes};

fn section(bytes: &[u8], kind: SectionType) -> std::ops::Range<usize> {
    let mut input = Cursor::new(bytes);
    let header = LlibHeader::read_from(&mut input).unwrap();
    input
        .seek(SeekFrom::Start(header.section_table_offset))
        .unwrap();
    for _ in 0..header.section_count {
        let entry = SectionEntry::read_from(&mut input).unwrap();
        if entry.section_type as u32 == kind as u32 {
            return entry.offset as usize..(entry.offset + entry.size) as usize;
        }
    }
    panic!("missing section");
}

#[test]
fn actual_objects_match_target_contract_and_mismatches_reject_before_use() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/artifact_target");
    let published = modes.artifact.join("target_provider");
    fs::create_dir(&published).unwrap();
    let provider = published.join("provider.ln");
    let artifact = provider.with_extension("llib");
    fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(&artifact)
        .env("LUNA_SYSROOT", &modes.artifact)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", render(&output));
    fs::remove_file(provider).unwrap();
    let baseline = fs::read(&artifact).unwrap();
    let manifest = LlibReader::read_manifest(&mut Cursor::new(&baseline)).unwrap();
    let header = LlibHeader::read_from(&mut Cursor::new(&baseline)).unwrap();
    assert_eq!(
        &header.target_triple[..manifest.target.target_triple.len()],
        manifest.target.target_triple.as_bytes()
    );
    let object = &baseline[section(&baseline, SectionType::ObjectCode)];
    assert!(!manifest.target.abi.is_empty());
    assert_eq!(manifest.target.pointer_width, usize::BITS as u8);
    assert_eq!(
        manifest.target.endianness,
        if cfg!(target_endian = "little") {
            "little"
        } else {
            "big"
        }
    );
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        assert_eq!(manifest.target.object_format, "COFF");
        assert_eq!(&object[..2], &[0x64, 0x86]);
    }
    #[cfg(target_os = "linux")]
    {
        assert_eq!(manifest.target.object_format, "ELF");
        assert_eq!(&object[..4], b"\x7fELF");
        assert_eq!(object[4], if usize::BITS == 64 { 2 } else { 1 });
    }
    #[cfg(target_os = "macos")]
    {
        assert_eq!(manifest.target.object_format, "MachO");
        assert_eq!(&object[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
    }
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("target_consumer");
        fs::create_dir(&project).unwrap();
        if mode == "source" {
            fs::copy(fixtures.join("provider.ln"), project.join("provider.ln")).unwrap();
        } else {
            fs::copy(&artifact, project.join("provider.llib")).unwrap();
        }
        fs::copy(fixtures.join("consumer.ln"), project.join("consumer.ln")).unwrap();
        let relocated = root.join("relocated_target_consumer");
        fs::rename(project, &relocated).unwrap();
        let (exe, build) = modes.build(
            root,
            &relocated.join("consumer.ln"),
            &format!("target_{mode}"),
        );
        assert!(build.status.success(), "{}", render(&build));
        assert_eq!(Command::new(exe).output().unwrap().status.code(), Some(0));
        eprintln!("PASS native/{mode}: relocated provider, exit0");
    }
    let project = modes.artifact.join("relocated_target_consumer");
    let destination = project.join("provider.llib");
    assert!(!project.join("provider.ln").exists());
    let manifest_range = section(&baseline, SectionType::Manifest);
    for (case, field, reason) in [
        (
            "triple",
            Some(manifest.target.target_triple.as_str()),
            "target triple mismatch",
        ),
        (
            "format",
            Some(manifest.target.object_format.as_str()),
            "target object format mismatch",
        ),
        (
            "cpu",
            Some(manifest.target.cpu.as_str()),
            "target CPU mismatch",
        ),
        (
            "features",
            Some(manifest.target.features.as_str()),
            "target features mismatch",
        ),
        (
            "abi",
            Some(manifest.target.abi.as_str()),
            "target ABI mismatch",
        ),
        (
            "endian",
            Some(manifest.target.endianness.as_str()),
            "target endianness mismatch",
        ),
        ("width", None, "target pointer width mismatch"),
        ("object", None, "embedded object target mismatch"),
        ("invalid_object", None, "invalid embedded object"),
        ("header", None, "TargetMismatch"),
        ("sidecar", None, "object sidecar content mismatch"),
    ] {
        let mut invalid = baseline.clone();
        fs::write(destination.with_extension("obj"), object).unwrap();
        if case == "sidecar" {
            let mut stale = object.to_vec();
            *stale.last_mut().unwrap() ^= 1;
            fs::write(destination.with_extension("obj"), stale).unwrap();
        } else if case == "header" {
            invalid[10] = b'X';
        } else if case == "object" || case == "invalid_object" {
            let range = section(&baseline, SectionType::ObjectCode);
            if case == "object" {
                // Preserve a valid object container but corrupt its architecture.
                let machine = match manifest.target.object_format.as_str() {
                    "COFF" => range.start..range.start + 2,
                    "ELF" => range.start + 18..range.start + 20,
                    "MachO" => range.start + 4..range.start + 8,
                    other => panic!("uncovered native format {other}"),
                };
                invalid[machine].fill(0);
            } else {
                invalid[range.start..range.start + 16].fill(0xff);
            }
        } else {
            let needle = field.unwrap_or(&manifest.target.abi).as_bytes();
            let relative = baseline[manifest_range.clone()]
                .windows(needle.len())
                .position(|value| value == needle)
                .unwrap();
            let offset = manifest_range.start + relative;
            if case == "width" {
                invalid[offset + needle.len()] = 1;
            } else {
                invalid[offset] = b'X';
            }
            if case == "triple" {
                invalid[10] = b'X';
            }
        }
        // Keep payload sizes/offsets and portable interfaces valid so each
        // rejection reaches the intended target/object validation boundary.
        fs::write(&destination, &invalid).unwrap();
        for command in ["check", "build"] {
            let exe = project.join(format!("rejected_{case}.exe"));
            let mut invocation = Command::new(env!("CARGO_BIN_EXE_luna"));
            invocation
                .arg(command)
                .arg(project.join("consumer.ln"))
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
                !output.status.success() && text.contains("error[E6001]") && text.contains(reason),
                "{case}/{command}: {text}"
            );
            assert!(!exe.exists());
            eprintln!("PASS reject/{case}/{command}: E6001, {reason}");
        }
    }
}
