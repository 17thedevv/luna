use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{CompilerOptions, check, compile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "luna_whole_file_io_{label}_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).expect("create whole-file I/O test directory");
    dir
}

fn canonical_external_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates directory")
        .parent()
        .expect("luna-rs directory")
        .join("libs")
        .join("external")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create copied sysroot directory");
    for entry in fs::read_dir(source).expect("enumerate sysroot") {
        let entry = entry.expect("read sysroot entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy sysroot item");
        }
    }
}

fn remove_extension(root: &Path, extension: &str) -> usize {
    let mut removed = 0;
    for entry in fs::read_dir(root).expect("enumerate sysroot for artifact removal") {
        let path = entry.expect("read sysroot entry").path();
        if path.is_dir() {
            removed += remove_extension(&path, extension);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            fs::remove_file(path).expect("remove sysroot artifact");
            removed += 1;
        }
    }
    removed
}

fn count_extension(root: &Path, extension: &str) -> usize {
    let mut count = 0;
    for entry in fs::read_dir(root).expect("enumerate sysroot for artifact count") {
        let path = entry.expect("read sysroot entry").path();
        if path.is_dir() {
            count += count_extension(&path, extension);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            count += 1;
        }
    }
    count
}

fn run_fixture(sysroot_root: &Path, run_dir: &Path) -> Output {
    fs::create_dir_all(run_dir).expect("create fixture run directory");
    let source_path = run_dir.join("whole_file_io_v1.ln");
    let executable = run_dir.join(if cfg!(windows) {
        "whole_file_io_v1.exe"
    } else {
        "whole_file_io_v1"
    });
    let source = include_str!("../../../tests/luna/stdlib/file/whole_file_io_v1.ln");
    fs::write(&source_path, source).expect("write Luna file-I/O fixture");

    let options = CompilerOptions {
        search_paths: vec![sysroot_root.to_string_lossy().to_string()],
        output_path: Some(executable.to_string_lossy().to_string()),
        quiet: true,
        ..Default::default()
    };
    let result = compile(source_path.to_str().unwrap(), source.to_string(), &options);
    if let Err(diagnostics) = result {
        let summary = diagnostics
            .iter()
            .filter(|diagnostic| !diagnostic.message.contains("E_UNCONSTRAINED_INFERENCE"))
            .take(80)
            .map(|diagnostic| format!("{:?}: {}", diagnostic.span, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n");
        panic!("Whole-File I/O fixture failed to compile:\n{summary}");
    }

    Command::new(&executable)
        .current_dir(run_dir)
        .output()
        .expect("run Whole-File I/O fixture")
}

fn assert_artifacts(dir: &Path, payload: &[u8]) {
    let mut mutated_payload = payload.to_vec();
    mutated_payload.push(68);
    let text = b"luna text\n";
    let large = (0..8_193)
        .map(|index| ((index * 31) % 256) as u8)
        .collect::<Vec<_>>();
    assert_eq!(fs::read(dir.join("written.bin")).unwrap(), mutated_payload);
    assert_eq!(fs::read(dir.join("copied.bin")).unwrap(), payload);
    assert_eq!(fs::read(dir.join("text-copy.bin")).unwrap(), text);
    assert_eq!(
        fs::read(dir.join("empty-copy.bin")).unwrap(),
        Vec::<u8>::new()
    );
    assert_eq!(fs::read(dir.join("large-copy.bin")).unwrap(), large);
    assert_eq!(fs::read(dir.join("text-written.bin")).unwrap(), text);
    assert_eq!(fs::read(dir.join("joined.bin")).unwrap(), payload);
    assert_eq!(fs::read(dir.join("empty.bin")).unwrap(), Vec::<u8>::new());
    assert_eq!(fs::read(dir.join("данные-✓.bin")).unwrap(), payload);
}

#[test]
fn whole_file_io_source_and_fresh_artifact_parity() {
    let root = temp_dir("parity");
    let source_external = canonical_external_dir();
    let build_root = root.join("build");
    let build_external = build_root.join("libs").join("external");
    copy_tree(&source_external, &build_external);

    let build_sysroot = Sysroot::from_root(build_root.clone()).expect("fresh builder sysroot");
    SysrootBuilder::new(build_sysroot)
        .build_all(true)
        .expect("build all current canonical providers into fresh artifacts");

    let source_root = root.join("source");
    let source_external = source_root.join("libs").join("external");
    copy_tree(&build_external, &source_external);
    assert!(remove_extension(&source_external, "llib") > 0);
    assert!(remove_extension(&source_external, "obj") > 0);
    assert_eq!(count_extension(&source_external, "llib"), 0);
    assert_eq!(count_extension(&source_external, "obj"), 0);

    let artifact_root = root.join("artifact");
    let artifact_external = artifact_root.join("libs").join("external");
    copy_tree(&build_external, &artifact_external);
    assert!(remove_extension(&artifact_external, "ln") > 0);
    assert_eq!(
        count_extension(&artifact_external, "ln"),
        0,
        "artifact-only mode must have no source fallback"
    );
    assert_eq!(count_extension(&artifact_external, "llib"), 36);
    assert_eq!(count_extension(&artifact_external, "obj"), 36);

    let payload = [0x00, 0xff, 0x80, 0x01, 0x7f, 0x00, 0xc3, 0xa9];
    let source_run = root.join("run_source");
    let artifact_run = root.join("run_artifact");
    let mut outputs = Vec::new();

    for (label, sysroot, run_dir) in [
        ("source", &source_root, &source_run),
        ("artifact", &artifact_root, &artifact_run),
    ] {
        fs::create_dir_all(run_dir).expect("create mode-specific runtime files");
        fs::write(run_dir.join("source.bin"), payload).unwrap();
        fs::write(run_dir.join("written.bin"), vec![0xaa; 128]).unwrap();
        fs::write(run_dir.join("copied.bin"), b"old copy destination").unwrap();
        fs::write(run_dir.join("text-source.bin"), b"luna text\n").unwrap();
        fs::write(run_dir.join("text-copy.bin"), b"old text destination").unwrap();
        fs::write(run_dir.join("empty-source.bin"), []).unwrap();
        fs::write(run_dir.join("empty-copy.bin"), b"old empty destination").unwrap();
        let large = (0..8_193)
            .map(|index| ((index * 31) % 256) as u8)
            .collect::<Vec<_>>();
        fs::write(run_dir.join("large-source.bin"), &large).unwrap();
        fs::write(run_dir.join("large-copy.bin"), b"old large destination").unwrap();
        fs::write(run_dir.join("text-written.bin"), b"old text output").unwrap();
        let output = run_fixture(sysroot, run_dir);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{label} fixture failed: stdout={:?}, stderr={}",
            output.stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "fixture should not emit output");
        assert_artifacts(run_dir, &payload);
        outputs.push(output);
    }

    assert_eq!(outputs[0].status.code(), outputs[1].status.code());
    assert_eq!(outputs[0].stdout, outputs[1].stdout);
    assert_eq!(outputs[0].stderr, outputs[1].stderr);
}

#[test]
fn copy_file_propagates_destination_write_failure_in_source_and_fresh_artifact_modes() {
    let root = temp_dir("copy_failure_parity");
    let build_external = root.join("build").join("libs").join("external");
    copy_tree(&canonical_external_dir(), &build_external);
    SysrootBuilder::new(
        Sysroot::from_root(root.join("build")).expect("fresh artifact builder sysroot"),
    )
    .build_all(true)
    .expect("build fresh canonical provider artifacts");

    let source_root = root.join("source");
    let source_external = source_root.join("libs").join("external");
    copy_tree(&build_external, &source_external);
    assert!(remove_extension(&source_external, "llib") > 0);
    assert!(remove_extension(&source_external, "obj") > 0);
    assert_eq!(count_extension(&source_external, "llib"), 0);
    assert_eq!(count_extension(&source_external, "obj"), 0);

    let artifact_root = root.join("artifact");
    let artifact_external = artifact_root.join("libs").join("external");
    copy_tree(&build_external, &artifact_external);
    assert!(remove_extension(&artifact_external, "ln") > 0);
    assert_eq!(count_extension(&artifact_external, "ln"), 0);
    assert_eq!(count_extension(&artifact_external, "llib"), 36);
    assert_eq!(count_extension(&artifact_external, "obj"), 36);

    let fixture = include_str!("../../../tests/luna/stdlib/file/copy_file_write_failure.ln");
    let mut outputs = Vec::new();
    for (label, sysroot) in [("source", &source_root), ("artifact", &artifact_root)] {
        let run_dir = root.join(format!("run_{label}"));
        fs::create_dir_all(&run_dir).expect("create isolated fixture directory");
        let source_path = run_dir.join("copy_file_write_failure.ln");
        let executable = run_dir.join(if cfg!(windows) {
            "copy_file_write_failure.exe"
        } else {
            "copy_file_write_failure"
        });
        fs::write(&source_path, fixture).expect("write copy failure fixture");

        let options = CompilerOptions {
            search_paths: vec![sysroot.to_string_lossy().to_string()],
            output_path: Some(executable.to_string_lossy().to_string()),
            quiet: true,
            ..Default::default()
        };
        compile(source_path.to_str().unwrap(), fixture.to_string(), &options).unwrap_or_else(
            |diagnostics| panic!("{label} copy-failure fixture did not compile: {diagnostics:#?}"),
        );
        let output = Command::new(&executable)
            .current_dir(&run_dir)
            .output()
            .unwrap_or_else(|error| panic!("could not run {label} fixture: {error}"));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{label} copy_file did not propagate destination write failure: stdout={:?}, stderr={}",
            output.stdout,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "unexpected {label} stdout");
        assert!(output.stderr.is_empty(), "unexpected {label} stderr");
        outputs.push(output);
    }

    assert_eq!(outputs[0].status.code(), outputs[1].status.code());
    assert_eq!(outputs[0].stdout, outputs[1].stdout);
    assert_eq!(outputs[0].stderr, outputs[1].stderr);
}

#[test]
fn whole_file_io_legacy_provider_and_root_paths_reject() {
    let sysroot = Sysroot::discover_for_test().expect("canonical sysroot");
    let source_path = temp_dir("namespace").join("reject.ln");
    let options = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        no_link: true,
        ..Default::default()
    };

    let negative_sources = [
        "import <file>; fn main() -> i32 { read_file(\"x\"); return 0; }",
        "import <file>; fn main() -> i32 { write_file(\"x\", \"\"); return 0; }",
        "import <file>; fn main() -> i32 { copy_file(\"x\", \"y\"); return 0; }",
        "import <file>; fn bad(value: FileError) -> i32 { return 0; } fn main() -> i32 { return 0; }",
        "import <file>; fn main() -> i32 { fs::read_file(\"x\"); return 0; }",
        "import <file>; fn main() -> i32 { fs::write_file(\"x\", \"\"); return 0; }",
        "import <file>; fn main() -> i32 { fs::copy_file(\"x\", \"y\"); return 0; }",
        "import <file>; fn main() -> i32 { std::fs::read_file(\"x\"); return 0; }",
        "import <file>; fn main() -> i32 { std::fs::write_file(\"x\", \"\"); return 0; }",
        "import <file>; fn main() -> i32 { std::fs::copy_file(\"x\", \"y\"); return 0; }",
    ];

    for (index, source) in negative_sources.iter().enumerate() {
        fs::write(&source_path, source).expect("write namespace-negative fixture");
        let result = check(source_path.to_str().unwrap(), source.to_string(), &options);
        assert!(
            result.is_err(),
            "legacy/private namespace form #{index} unexpectedly resolved: {source}"
        );
    }
}
