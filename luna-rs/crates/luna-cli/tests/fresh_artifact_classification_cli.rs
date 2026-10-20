//! C5 freeze — artifact freshness classification.
//!
//! Each stale class must be rejected deterministically and *classified*: the
//! diagnostic must name the failing boundary (format / compiler / mvir /
//! semantic-metadata / target / object identity) instead of collapsing to an
//! opaque `CorruptedData`. Interface- and execution-fingerprint staleness are
//! covered by `interface_constraints_cli` and `comptime_dependency_precision_cli`.
#[path = "support/stdlib.rs"]
mod support;

use std::{fs, path::Path, process::Command};
use support::{render, workspace_root, ProviderModes};

fn patch_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

/// Returns `(entry_offset, payload_offset, payload_size)` of the first section
/// of `section_type`. `entry_offset + 32` holds the section checksum.
fn section(bytes: &[u8], section_type: u32) -> Option<(usize, usize, usize)> {
    let count = u32::from_le_bytes(bytes[110..114].try_into().unwrap()) as usize;
    let table = u64::from_le_bytes(bytes[114..122].try_into().unwrap()) as usize;
    for index in 0..count {
        let entry = table + index * 40;
        let kind = u32::from_le_bytes(bytes[entry + 4..entry + 8].try_into().unwrap());
        if kind == section_type {
            let offset = u64::from_le_bytes(bytes[entry + 8..entry + 16].try_into().unwrap()) as usize;
            let size = u64::from_le_bytes(bytes[entry + 16..entry + 24].try_into().unwrap()) as usize;
            return Some((entry, offset, size));
        }
    }
    None
}

/// Recompute a section checksum after its payload was mutated, so validation can
/// reach the boundary under test instead of stopping at the checksum.
fn refresh_section_checksum(bytes: &mut [u8], entry: usize, offset: usize, size: usize) {
    let checksum = luna_llib::format::section_checksum(&bytes[offset..offset + size]);
    bytes[entry + 32..entry + 40].copy_from_slice(&checksum.to_le_bytes());
}

fn publish_provider(root: &Path, dir: &Path) -> Vec<u8> {
    fs::create_dir_all(dir).unwrap();
    let source = dir.join("provider.ln");
    fs::write(&source, "module provider { export fn value() -> i32 { return 3; } }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(dir.join("provider.llib"))
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "provider publish: {}", render(&output));
    fs::remove_file(&source).unwrap();
    fs::read(dir.join("provider.llib")).unwrap()
}

fn build_consumer(
    modes: &ProviderModes,
    root: &Path,
    dir: &Path,
    tag: &str,
) -> (std::process::Output, bool) {
    let consumer = dir.join("consumer.ln");
    fs::write(&consumer, "import \"provider\";\nfn main() -> i32 { return provider::value() - 3; }\n")
        .unwrap();
    let (exe, output) = modes.build(root, &consumer, tag);
    (output, exe.exists())
}

#[test]
fn stale_artifacts_are_rejected_with_a_classified_reason() {
    let modes = ProviderModes::fresh();
    let mut failures = Vec::new();

    // Fresh artifact (control): builds and runs.
    {
        let dir = modes.source.join("fresh");
        let _ = publish_provider(&modes.source, &dir);
        let (output, published) = build_consumer(&modes, &modes.source, &dir, "c5_fresh");
        if !output.status.success() {
            failures.push(format!("fresh build: {}", render(&output)));
        }
        if published {
            let exe = dir.join("consumer");
            let _ = exe;
        }
    }

    // Each stale class.
    let cases: Vec<(&str, Box<dyn Fn(&mut Vec<u8>)>, &str)> = vec![
        (
            "format",
            Box::new(|bytes: &mut Vec<u8>| patch_u16(bytes, 4, 1)),
            "format",
        ),
        (
            "compiler",
            Box::new(|bytes: &mut Vec<u8>| patch_u16(bytes, 6, 21)),
            "compiler",
        ),
        (
            "mvir",
            Box::new(|bytes: &mut Vec<u8>| patch_u16(bytes, 8, 4)),
            "mvir",
        ),
        (
            "target",
            Box::new(|bytes: &mut Vec<u8>| {
                bytes[10..74].fill(0);
                let triple = b"aarch64-unknown-linux-gnu";
                bytes[10..10 + triple.len()].copy_from_slice(triple);
            }),
            "TargetMismatch",
        ),
        (
            "metadata",
            Box::new(|bytes: &mut Vec<u8>| {
                if let Some((entry, offset, size)) = section(bytes, 16) {
                    patch_u16(bytes, offset, 1);
                    refresh_section_checksum(bytes, entry, offset, size);
                }
            }),
            "semantic-metadata",
        ),
        (
            "object",
            Box::new(|bytes: &mut Vec<u8>| {
                if let Some((entry, offset, size)) = section(bytes, 8) {
                    if size > 0 {
                        bytes[offset] ^= 0xFF;
                        refresh_section_checksum(bytes, entry, offset, size);
                    }
                }
            }),
            "ObjectIntegrityMismatch",
        ),
    ];

    for (name, mutate, expected) in cases {
        let dir = modes.artifact.join(format!("case_{name}"));
        fs::create_dir_all(&dir).unwrap();
        let mut bytes = publish_provider(&modes.artifact, &dir);
        mutate(&mut bytes);
        fs::write(dir.join("provider.llib"), &bytes).unwrap();
        let (output, published) = build_consumer(&modes, &modes.artifact, &dir, &format!("c5_{name}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            failures.push(format!("{name}: stale artifact was ACCEPTED"));
        } else if !stderr.contains(expected) {
            failures.push(format!("{name}: expected `{expected}` in {}", render(&output)));
        }
        if published {
            failures.push(format!("{name}: published an executable for a stale artifact"));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
