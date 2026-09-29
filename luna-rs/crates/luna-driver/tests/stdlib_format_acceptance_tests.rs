use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{check_semantic_only, CompilerOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn canonical_external() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external")
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "luna_fmt_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn remove_extension_recursively(root: &Path, extension: &str) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            remove_extension_recursively(&path, extension);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            fs::remove_file(path).unwrap();
        }
    }
}

fn run_fixture(root: &Path, fixture: &str, tag: &str) -> (i32, String, String) {
    let work = temp_root(tag);
    fs::create_dir_all(&work).unwrap();
    let source = work.join("formatting.ln");
    let executable = work.join("formatting.exe");
    fs::write(&source, fixture).unwrap();
    let options = CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        no_link: false,
        ..Default::default()
    };
    luna_driver::compile(&source.to_string_lossy(), fixture.to_string(), &options)
        .unwrap_or_else(|diagnostics| panic!("format fixture must compile: {diagnostics:#?}"));
    let output = Command::new(executable).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn formatting_v1_matches_source_and_fresh_artifact_sysroots() {
    let fixture = include_str!("../../../tests/luna/stdlib/core/formatting_v1.ln");
    let artifacts = temp_root("fresh_artifacts");
    copy_tree(&canonical_external(), &artifacts.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifacts.clone()).unwrap())
        .build_all(true)
        .expect("fresh .llib/.obj providers must build from current source");

    let source_only = temp_root("source_only");
    copy_tree(
        &artifacts.join("libs/external"),
        &source_only.join("libs/external"),
    );
    remove_extension_recursively(&source_only, "llib");
    remove_extension_recursively(&source_only, "obj");

    let artifact_only = temp_root("artifact_only");
    copy_tree(
        &artifacts.join("libs/external"),
        &artifact_only.join("libs/external"),
    );
    remove_extension_recursively(&artifact_only, "ln");

    let source_result = run_fixture(&source_only, fixture, "source_only");
    let artifact_result = run_fixture(&artifact_only, fixture, "artifact_only");
    assert_eq!(
        source_result, artifact_result,
        "source and .llib output must agree"
    );
    assert_eq!(source_result.0, 0, "format run failed: {source_result:?}");
    assert_eq!(source_result.1.replace("\r\n", "\n"), "true|false|0|12345|123456789|18446744073709551615|123|12345|-128|-12345|-123456789|-123456789|-123|-12345|界|Việt|-12,34\n");
    assert!(source_result.2.is_empty());
}

#[test]
fn formatting_v1_does_not_claim_display_for_bytes_or_floats() {
    let sysroot = Sysroot::discover_for_test().unwrap();
    let work = temp_root("negative");
    fs::create_dir_all(&work).unwrap();
    let source = work.join("negative.ln");
    let fixture = r#"
import <fmt>;
fn require_display<T: std::fmt::Display>(value: &T) {}
fn main() {
    dec bytes: [u8; 1] = [65 as u8];
    require_display(&bytes);
    require_display(1.5 as f64);
}
"#;
    fs::write(&source, fixture).unwrap();
    let options = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().into_owned()],
        quiet: true,
        ..Default::default()
    };
    let result = check_semantic_only(&source.to_string_lossy(), fixture.to_string(), &options);
    assert!(
        result.is_err(),
        "arbitrary bytes and floats are outside Display v1"
    );
}
