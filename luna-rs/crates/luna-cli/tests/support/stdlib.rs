//! Fresh, isolated providers exercised only through the public compiler CLI.
#![allow(dead_code)] // Individual CLI suites use different parts of this support module.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static PROVIDER_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

pub struct ProviderModes {
    work: PathBuf,
    pub source: PathBuf,
    pub artifact: PathBuf,
}

fn copy_selected(source: &Path, destination: &Path, extensions: &[&str]) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        let target = destination.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_selected(&path, &target, extensions);
        } else if extensions.contains(&path.extension().and_then(|ext| ext.to_str()).unwrap_or(""))
        {
            fs::copy(path, target).unwrap();
        }
    }
}

fn count_extension(root: &Path, extension: &str) -> usize {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                count_extension(&path, extension)
            } else {
                usize::from(path.extension().and_then(|ext| ext.to_str()) == Some(extension))
            }
        })
        .sum()
}

pub fn render(output: &Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

impl ProviderModes {
    pub fn fresh() -> Self {
        Self::fresh_with_source_edit(|_| {})
    }

    pub fn fresh_with_source_edit(edit: impl FnOnce(&Path)) -> Self {
        let work = std::env::temp_dir().join(format!(
            "luna_cli_parity_{}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            PROVIDER_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&work).unwrap();
        let modes = Self {
            source: work.join("source"),
            artifact: work.join("artifact"),
            work,
        };
        // POSIX executable names have no suffix; keep them separate from roots
        // named `source` / `artifact`, rather than relying on Windows `.exe`.
        fs::create_dir(modes.work.join("executables")).unwrap();
        let built = modes.work.join("built");
        // Do not copy any pre-existing .llib/.obj or build cache.
        copy_selected(
            &workspace_root().join("libs/external"),
            &built.join("libs/external"),
            &["ln", "toml"],
        );
        edit(&built.join("libs/external"));
        assert_eq!(count_extension(&built, "llib"), 0);
        assert_eq!(count_extension(&built, "obj"), 0);
        let build = Command::new(env!("CARGO_BIN_EXE_luna"))
            .args(["build-sysroot", "--quiet"])
            .env("LUNA_SYSROOT", &built)
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "fresh CLI sysroot build failed: {}",
            render(&build)
        );
        copy_selected(
            &built.join("libs/external"),
            &modes.source.join("libs/external"),
            &["ln", "toml"],
        );
        copy_selected(
            &built.join("libs/external"),
            &modes.artifact.join("libs/external"),
            &["llib", "obj", "toml"],
        );
        assert_eq!(count_extension(&modes.source, "llib"), 0);
        assert_eq!(count_extension(&modes.source, "obj"), 0);
        assert_eq!(count_extension(&modes.artifact, "ln"), 0);
        assert!(count_extension(&modes.source, "ln") > 0);
        assert_eq!(
            count_extension(&modes.source, "ln"),
            count_extension(&modes.artifact, "llib")
        );
        modes
    }

    pub fn build(&self, root: &Path, fixture: &Path, tag: &str) -> (PathBuf, Output) {
        assert!(!tag.contains('/') && !tag.contains('\\'));
        let executable = self
            .work
            .join("executables")
            .join(tag)
            .with_extension(std::env::consts::EXE_EXTENSION);
        let output = Command::new(env!("CARGO_BIN_EXE_luna"))
            .arg("build")
            .arg(fixture)
            .arg("-o")
            .arg(&executable)
            .arg("--quiet")
            .arg("-I")
            .arg(root)
            .env("LUNA_SYSROOT", root)
            .output()
            .unwrap();
        (executable, output)
    }

    pub fn check(&self, root: &Path, fixture: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_luna"))
            .arg("check")
            .arg(fixture)
            .arg("--quiet")
            .arg("-I")
            .arg(root)
            .env("LUNA_SYSROOT", root)
            .output()
            .unwrap()
    }
}

impl Drop for ProviderModes {
    fn drop(&mut self) {
        // Only the unique directory created above is owned by this harness.
        if self.work.parent() == Some(std::env::temp_dir().as_path())
            && self
                .work
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("luna_cli_parity_")
        {
            let _ = fs::remove_dir_all(&self.work);
        }
    }
}

fn normalize_type_ids(message: &str) -> String {
    let mut normalized = String::new();
    let mut remaining = message;
    while let Some(start) = remaining.find("SemanticTypeId(") {
        normalized.push_str(&remaining[..start]);
        let suffix = &remaining[start + "SemanticTypeId(".len()..];
        if let Some(end) = suffix.find(')') {
            if suffix[..end].bytes().all(|byte| byte.is_ascii_digit()) {
                normalized.push_str("SemanticTypeId(<session>)");
                remaining = &suffix[end + 1..];
                continue;
            }
        }
        normalized.push_str(&remaining[start..]);
        return normalized;
    }
    normalized.push_str(remaining);
    normalized
}

pub fn diagnostic_messages(output: &Output) -> Vec<String> {
    // `check` prints messages while `build` also renders source snippets.
    // Compare semantic reasons, not those deliberately different presentations.
    let mut messages: Vec<_> = std::str::from_utf8(&output.stderr)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("error:") || line.starts_with("error["))
        .map(normalize_type_ids)
        .collect();
    assert!(!messages.is_empty(), "{}", render(output));
    messages.sort();
    messages
}
