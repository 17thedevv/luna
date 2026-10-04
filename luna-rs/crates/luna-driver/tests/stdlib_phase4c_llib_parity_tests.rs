use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{compile, CompilerOptions};

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn remove_files_by_ext(dir: &Path, ext: &str) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_files_by_ext(&path, ext);
            } else if path.extension().is_some_and(|value| value == ext) {
                fs::remove_file(path).expect("remove isolated sysroot artifact");
            }
        }
    }
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "luna_phase4c_llib_{}_{}_{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create isolated parity directory");
    dir
}

struct Phase4cParityHarness {
    source_sysroot: PathBuf,
    artifact_sysroot: PathBuf,
}

impl Phase4cParityHarness {
    fn new() -> Self {
        let repository_sysroot = Sysroot::discover_for_test().expect("canonical sysroot required");
        let repository_external = repository_sysroot.external_dir();
        let base = create_temp_dir("sysroots");
        let source_root = base.join("source");
        let artifact_root = base.join("artifact");
        let source_external = source_root.join("libs").join("external");
        let artifact_external = artifact_root.join("libs").join("external");
        copy_dir_all(repository_external, &source_external).expect("copy source sysroot");
        copy_dir_all(repository_external, &artifact_external).expect("copy artifact input sysroot");

        // Keep only source providers in A.
        remove_files_by_ext(&source_external, "llib");
        remove_files_by_ext(&source_external, "obj");

        // Rebuild every artifact from the isolated source copy, then remove every source file
        // so B cannot silently fall back to `.ln`.
        let artifact_sysroot = Sysroot::discover(Some(artifact_root.to_string_lossy().as_ref()))
            .expect("discover isolated artifact sysroot");
        SysrootBuilder::new(artifact_sysroot)
            .build_all(true)
            .expect("build fresh isolated .llib/.obj sysroot");
        remove_files_by_ext(&artifact_external, "ln");
        assert_eq!(
            count_files_by_ext(&artifact_external, "ln"),
            0,
            "artifact sysroot must contain no Luna source files"
        );

        Self { source_sysroot: source_root, artifact_sysroot: artifact_root }
    }

    fn run(&self, mode: &str, sysroot: &Path, source: &str) -> (i32, String, String) {
        let dir = create_temp_dir(mode);
        let source_path = dir.join("main.ln");
        let executable = dir.join(if cfg!(windows) { "main.exe" } else { "main" });
        fs::write(&source_path, source).expect("write parity consumer");
        let options = CompilerOptions {
            search_paths: vec![sysroot.to_string_lossy().to_string()],
            output_path: Some(executable.to_string_lossy().to_string()),
            quiet: true,
            ..Default::default()
        };
        compile(source_path.to_str().unwrap(), source.to_string(), &options)
            .unwrap_or_else(|diags| panic!("{mode} parity compilation failed: {diags:?}"));
        let output = Command::new(executable).output().expect("execute parity consumer");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    fn assert_parity(&self, source: &str) {
        let source_result = self.run("source", &self.source_sysroot, source);
        let artifact_result = self.run("artifact_only", &self.artifact_sysroot, source);
        assert_eq!(source_result.0, 0, "source consumer failed: {:?}", source_result);
        assert_eq!(artifact_result.0, 0, "artifact-only consumer failed: {:?}", artifact_result);
        assert_eq!(source_result, artifact_result, "Phase 4C source/.llib observable behavior differs");
    }
}

fn count_files_by_ext(dir: &Path, ext: &str) -> usize {
    fs::read_dir(dir)
        .expect("read isolated sysroot directory")
        .filter_map(Result::ok)
        .map(|entry| {
            if entry.path().is_dir() {
                count_files_by_ext(&entry.path(), ext)
            } else if entry.path().extension().is_some_and(|value| value == ext) {
                1
            } else {
                0
            }
        })
        .sum()
}

#[test]
fn phase4c_fresh_session_source_vs_llib_parity() {
    let harness = Phase4cParityHarness::new();
    let fixture = r#"
        import <convert>;
        import <result>;
        import <default>;
        import <copy>;
        import <vec>;
        import <hashmap>;
        import <float>;

        fn reflexive<T: std::Convert<T>>(value: T) -> T { return value.convert(); }

        fn main() -> i32 {
            // Concrete widening and reflexive blanket selection.
            dec small: u8 = 42 as u8;
            dec widened: u16 = std::Convert::convert(small);
            if widened != (42 as u16) { return 1; }
            dec same: u8 = reflexive<u8>(small);
            if same != small { return 2; }

            // std::TryConvert success and all three typed error classes.
            dec boundary: u16 = 255 as u16;
            dec narrow: std::Result<u8, std::TryConvertError> = std::TryConvert::try_convert(boundary);
            match narrow {
                std::Result::Ok(value) -> { if value != (255 as u8) { return 3; } },
                std::Result::Err(_) -> { return 4; },
            }
            dec too_large: u16 = 256 as u16;
            dec overflow: std::Result<u8, std::TryConvertError> = std::TryConvert::try_convert(too_large);
            match overflow {
                std::Result::Err(std::TryConvertError::Overflow) -> {},
                _ -> { return 5; },
            }
            dec negative: i32 = -1;
            dec underflow: std::Result<u32, std::TryConvertError> = std::TryConvert::try_convert(negative);
            match underflow {
                std::Result::Err(std::TryConvertError::Underflow) -> {},
                _ -> { return 6; },
            }
            dec nan = std::f32_nan();
            dec invalid: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(nan);
            match invalid {
                std::Result::Err(std::TryConvertError::Invalid) -> {},
                _ -> { return 7; },
            }

            // Default on a primitive, std::Option, Vec and HashMap.
            dec zero: i32 = std::Default::default();
            if zero != 0 { return 8; }
            dec maybe: std::Option<i32> = std::Default::default();
            if maybe.is_some() { return 9; }
            dec values: std::Vec<i32> = std::Default::default();
            if values.len() != (0 as u64) { return 10; }
            dec map: std::HashMap<i32, i32> = std::Default::default();
            if map.len() != (0 as u64) { return 11; }

            // Ordinary FloatOps dispatch and bit-preserving factories.
            if ((0.0 as f32) - (2.0 as f32)).abs() != (2.0 as f32) { return 12; }
            if (1.0 as f32).min(2.0 as f32) != (1.0 as f32) { return 13; }
            if (1.0 as f64).max(2.0 as f64) != (2.0 as f64) { return 14; }
            if std::f64_from_bits(std::f64_to_bits(1.5 as f64)) != (1.5 as f64) { return 15; }
            if std::f64_nan().is_nan() == false { return 16; }
            return 0;
        }
    "#;
    harness.assert_parity(fixture);
}
