use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{compile, CompilerOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn create_temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "luna_path_semantics_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create path test temp directory");
    dir
}

fn canonical_external_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs")
        .join("external")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create isolated sysroot directory");
    for entry in fs::read_dir(source).expect("enumerate sysroot directory") {
        let entry = entry.expect("read sysroot entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy sysroot file");
        }
    }
}

fn remove_files_with_extension(root: &Path, extension: &str) -> usize {
    let mut removed = 0;
    for entry in fs::read_dir(root).expect("enumerate isolated sysroot") {
        let path = entry.expect("read isolated sysroot entry").path();
        if path.is_dir() {
            removed += remove_files_with_extension(&path, extension);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            fs::remove_file(path).expect("remove isolated sysroot artifact");
            removed += 1;
        }
    }
    removed
}

fn count_files_with_extension(root: &Path, extension: &str) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(root).expect("enumerate isolated sysroot") {
        let path = entry.expect("read isolated sysroot entry").path();
        if path.is_dir() {
            count += count_files_with_extension(&path, extension);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            count += 1;
        }
    }
    count
}

fn run_fixture(
    mode: &str,
    sysroot_root: &Path,
    source: &str,
    run_dir: &Path,
) -> (i32, Vec<u8>, Vec<u8>) {
    fs::create_dir_all(run_dir).expect("create fixture directory");
    let source_path = run_dir.join("main.ln");
    let executable = run_dir.join(if cfg!(windows) { "main.exe" } else { "main" });
    fs::write(&source_path, source).expect("write path acceptance fixture");

    let options = CompilerOptions {
        search_paths: vec![sysroot_root.to_string_lossy().to_string()],
        output_path: Some(executable.to_string_lossy().to_string()),
        quiet: true,
        ..Default::default()
    };
    let result = compile(source_path.to_str().unwrap(), source.to_string(), &options);
    assert!(
        result.is_ok(),
        "path fixture must compile in {mode} mode: {:?}",
        result.err()
    );

    let output = Command::new(&executable)
        .output()
        .expect("run path acceptance fixture");
    (
        output.status.code().unwrap_or(-1),
        output.stdout,
        output.stderr,
    )
}

#[test]
fn path_provider_source_llib_loading_parity() {
    let dir = create_temp_dir();
    let build_root = dir.join("build");
    let build_external = build_root.join("libs").join("external");
    copy_tree(&canonical_external_dir(), &build_external);

    // Build canonical provider artifacts only in this isolated sysroot.
    let build_sysroot = Sysroot::from_root(build_root.clone()).expect("isolated sysroot");
    SysrootBuilder::new(build_sysroot)
        .build_all(true)
        .expect("build isolated canonical providers");

    // Exercise path/path.ln with its own .llib and .obj absent; dependencies
    // remain canonical artifacts so this isolates the path provider boundary.
    let source_root = dir.join("source");
    let source_external = source_root.join("libs").join("external");
    copy_tree(&build_external, &source_external);
    fs::remove_file(source_external.join("path").join("path.llib"))
        .expect("remove path artifact from source mode");
    fs::remove_file(source_external.join("path").join("path.obj"))
        .expect("remove path object from source mode");

    let artifact_root = dir.join("artifact");
    let artifact_external = artifact_root.join("libs").join("external");
    copy_tree(&build_external, &artifact_external);
    remove_files_with_extension(&artifact_external, "ln");

    assert!(source_external.join("path").join("path.ln").exists());
    assert!(!source_external.join("path").join("path.llib").exists());
    assert_eq!(count_files_with_extension(&source_external, "llib"), 31);
    assert_eq!(count_files_with_extension(&artifact_external, "ln"), 0);
    assert_eq!(count_files_with_extension(&artifact_external, "llib"), 32);
    assert_eq!(count_files_with_extension(&artifact_external, "obj"), 32);

    let source = include_str!("../../../tests/luna/stdlib/path/path_semantics_v1.ln");
    let source_output = run_fixture("source", &source_root, source, &dir.join("run_source"));
    let artifact_output = run_fixture(
        "artifact",
        &artifact_root,
        source,
        &dir.join("run_artifact"),
    );

    assert_eq!(
        source_output.0,
        0,
        "source mode failed: {}",
        String::from_utf8_lossy(&source_output.2)
    );
    assert_eq!(
        artifact_output.0,
        0,
        "artifact mode failed: {}",
        String::from_utf8_lossy(&artifact_output.2)
    );
    assert_eq!(source_output.0, artifact_output.0, "source/artifact exit-code mismatch");
    assert_eq!(source_output.1, artifact_output.1, "source/artifact stdout mismatch");
    assert_eq!(source_output.2, artifact_output.2, "source/artifact stderr mismatch");
    assert!(source_output.1.is_empty(), "path fixture should not print output");
    assert!(artifact_output.1.is_empty(), "path fixture should not print output");
}

#[test]
fn path_source_string_source_artifact_parity_regression() {
    let dir = create_temp_dir();
    let build_root = dir.join("build");
    let build_external = build_root.join("libs").join("external");
    copy_tree(&canonical_external_dir(), &build_external);
    let build_sysroot = Sysroot::from_root(build_root.clone()).expect("isolated sysroot");
    SysrootBuilder::new(build_sysroot)
        .build_all(true)
        .expect("build isolated canonical providers");

    // Source mode forces both providers through the source semantic pipeline;
    // all .llib and .obj artifacts for these providers are absent.
    let source_root = dir.join("source");
    let source_external = source_root.join("libs").join("external");
    copy_tree(&build_external, &source_external);
    for provider in ["path/path", "alloc/string"] {
        let artifact_base = source_external.join(provider);
        fs::remove_file(artifact_base.with_extension("llib"))
            .unwrap_or_else(|e| panic!("remove {provider} llib: {e}"));
        fs::remove_file(artifact_base.with_extension("obj"))
            .unwrap_or_else(|e| panic!("remove {provider} obj: {e}"));
    }

    let artifact_root = dir.join("artifact");
    let artifact_external = artifact_root.join("libs").join("external");
    copy_tree(&build_external, &artifact_external);
    remove_files_with_extension(&artifact_external, "ln");

    let source = include_str!("../../../tests/luna/stdlib/path/source_closure_borrow_conflict.ln");
    let source_output = run_fixture("source", &source_root, source, &dir.join("run_source"));
    let artifact_result = run_fixture("artifact", &artifact_root, source, &dir.join("run_artifact"));
    assert_eq!(source_output.0, 0, "source mode failed: {}", String::from_utf8_lossy(&source_output.2));
    assert_eq!(
        artifact_result.0, 0,
        "artifact mode failed: {}",
        String::from_utf8_lossy(&artifact_result.2)
    );
    assert_eq!(source_output.0, artifact_result.0, "source/artifact exit-code mismatch");
    assert_eq!(source_output.1, artifact_result.1, "source/artifact stdout mismatch");
    assert_eq!(source_output.2, artifact_result.2, "source/artifact stderr mismatch");
}
