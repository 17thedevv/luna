use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn SetErrorMode(mode: u32) -> u32;
}

#[cfg(windows)]
struct WindowsErrorModeGuard(u32);

#[cfg(windows)]
impl WindowsErrorModeGuard {
    fn suppress_crash_dialogs() -> Self {
        const SEM_FAILCRITICALERRORS: u32 = 0x0001;
        const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;
        Self(unsafe { SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX) })
    }
}

#[cfg(windows)]
impl Drop for WindowsErrorModeGuard {
    fn drop(&mut self) {
        unsafe {
            SetErrorMode(self.0);
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("luna-cli must live below the workspace root")
        .to_path_buf()
}

fn unique_temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "luna_char_{name}_{}_{}",
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

fn count_extension_recursively(root: &Path, extension: &str) -> usize {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                count_extension_recursively(&path, extension)
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                1
            } else {
                0
            }
        })
        .sum()
}

fn fresh_provider_modes() -> (PathBuf, PathBuf) {
    let root = workspace_root();
    let source_external = root.join("libs/external");
    let built_root = unique_temp("fresh_build");
    copy_tree(&source_external, &built_root.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(built_root.clone()).unwrap())
        .build_all(true)
        .expect("fresh provider artifacts must build from current source");

    let source_only = unique_temp("source_only");
    copy_tree(
        &built_root.join("libs/external"),
        &source_only.join("libs/external"),
    );
    remove_extension_recursively(&source_only, "llib");
    remove_extension_recursively(&source_only, "obj");
    assert_eq!(count_extension_recursively(&source_only, "llib"), 0);
    assert_eq!(count_extension_recursively(&source_only, "obj"), 0);

    let artifact_only = unique_temp("artifact_only");
    copy_tree(
        &built_root.join("libs/external"),
        &artifact_only.join("libs/external"),
    );
    remove_extension_recursively(&artifact_only, "ln");
    assert_eq!(count_extension_recursively(&artifact_only, "ln"), 0);

    (source_only, artifact_only)
}

fn fixture_path(relative: &str) -> PathBuf {
    workspace_root().join("tests/luna").join(relative)
}

fn build_with_cli(root: &Path, fixture: &Path, tag: &str) -> (PathBuf, Output) {
    let safe_tag = tag.replace('/', "_").replace('\\', "_");
    let executable = unique_temp(&safe_tag).with_extension(std::env::consts::EXE_EXTENSION);
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(fixture)
        .arg("-o")
        .arg(&executable)
        .arg("--quiet")
        .arg("-I")
        .arg(root)
        .output()
        .expect("public luna build command must start");
    (executable, output)
}

fn compile_diagnostics(root: &Path, fixture: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("check")
        .arg(fixture)
        .arg("--quiet")
        .arg("-I")
        .arg(root)
        .output()
        .expect("public luna check command must start")
}

fn render(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn char_scalar_contract_and_formatting_match_fresh_source_and_artifacts() {
    #[cfg(windows)]
    let _error_mode = WindowsErrorModeGuard::suppress_crash_dialogs();

    let (source_only, artifact_only) = fresh_provider_modes();

    let valid = fixture_path("stdlib/core/formatting_char_cast_boundaries_v1.ln");
    let mut results = Vec::new();
    for (tag, root) in [("source", &source_only), ("artifact", &artifact_only)] {
        let (executable, build) = build_with_cli(root, &valid, tag);
        assert!(
            build.status.success(),
            "{tag} build failed: {}",
            render(&build)
        );
        let run = Command::new(&executable)
            .output()
            .expect("compiled char boundary program must execute");
        let _ = fs::remove_file(executable);
        assert!(run.status.success(), "{tag} run failed: {}", render(&run));
        results.push((run.status.code(), run.stdout, run.stderr));
    }

    let mut expected = vec![
        0x00, 0x7F, 0xC2, 0x80, 0xED, 0x9F, 0xBF, 0xEE, 0x80, 0x80, 0xF4, 0x8F, 0xBF, 0xBF,
    ];
    if cfg!(windows) {
        expected.extend_from_slice(b"\r\n");
    } else {
        expected.push(b'\n');
    }
    assert_eq!(
        results[0], results[1],
        "source and fresh artifact CLI behavior diverged"
    );
    assert_eq!(results[0].0, Some(0));
    assert_eq!(results[0].1, expected);
    assert!(results[0].2.is_empty());

    let existing_unicode = fixture_path("stdlib/core/formatting_unicode_v1.ln");
    let mut unicode_outputs = Vec::new();
    for (tag, root) in [
        ("source_unicode", &source_only),
        ("artifact_unicode", &artifact_only),
    ] {
        let (executable, build) = build_with_cli(root, &existing_unicode, tag);
        assert!(
            build.status.success(),
            "{tag} build failed: {}",
            render(&build)
        );
        let run = Command::new(&executable)
            .output()
            .expect("Unicode fixture must execute");
        let _ = fs::remove_file(executable);
        assert!(run.status.success(), "{tag} run failed: {}", render(&run));
        unicode_outputs.push((run.status.code(), run.stdout, run.stderr));
    }
    assert_eq!(unicode_outputs[0], unicode_outputs[1]);

    let invalid = [
        (
            "compiler/char_cast_invalid_comptime_surrogate.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_invalid_comptime_surrogate_end.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_invalid_comptime_above_max.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_invalid_comptime_u32_max.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_invalid_comptime_u128_max.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_invalid_comptime_negative.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_invalid_comptime_signed_widen.ln",
            "Unicode scalar",
        ),
        (
            "compiler/char_cast_float_rejected.ln",
            "source must be an integer or char",
        ),
        (
            "compiler/char_cast_bool_rejected.ln",
            "source must be an integer or char",
        ),
        (
            "compiler/char_cast_pointer_rejected.ln",
            "source must be an integer or char",
        ),
    ];
    for (relative, expected_diagnostic) in invalid {
        let path = fixture_path(relative);
        let source = compile_diagnostics(&source_only, &path);
        let artifact = compile_diagnostics(&artifact_only, &path);
        assert!(
            !source.status.success(),
            "source accepted {relative}: {}",
            render(&source)
        );
        assert!(
            !artifact.status.success(),
            "artifact mode accepted {relative}: {}",
            render(&artifact)
        );
        assert_eq!(
            source.stderr, artifact.stderr,
            "diagnostics diverged for {relative}"
        );
        assert!(
            String::from_utf8_lossy(&source.stderr).contains(expected_diagnostic),
            "missing expected diagnostic for {relative}: {}",
            render(&source)
        );
    }

    for relative in [
        "compiler/char_cast_invalid_runtime_surrogate.ln",
        "compiler/char_cast_invalid_runtime_above_max.ln",
        "compiler/char_cast_invalid_runtime_negative.ln",
        "compiler/char_cast_invalid_runtime_u16_surrogate.ln",
        "compiler/char_cast_invalid_runtime_i8_negative.ln",
        "compiler/char_cast_invalid_runtime_u128_high_bit.ln",
    ] {
        let path = fixture_path(relative);
        let mut outcomes = Vec::new();
        for (tag, root) in [("source", &source_only), ("artifact", &artifact_only)] {
            let (executable, build) = build_with_cli(root, &path, &format!("{tag}_{relative}"));
            assert!(
                build.status.success(),
                "{tag} must compile dynamic cast: {}",
                render(&build)
            );
            let status = Command::new(&executable)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("dynamic invalid char cast must execute to its guard");
            let _ = fs::remove_file(executable);
            assert!(
                !status.success(),
                "invalid dynamic char cast returned normally"
            );
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(
                    status.signal(),
                    Some(6),
                    "invalid cast must abort, not crash or return: {relative}"
                );
            }
            #[cfg(windows)]
            assert!(
                matches!(status.code(), Some(3 | 101 | 102 | -1073740791)),
                "invalid cast must use the runtime abort path: {relative}: {status}"
            );
            outcomes.push(status.code());
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "runtime guard diverged for {relative}"
        );
    }
}
