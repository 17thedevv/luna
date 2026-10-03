use luna_driver::{lang_contracts::LangContractManifest, sysroot_manifest::SysrootManifest};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
fn render(output: &Output) -> String {
    format!(
        "{}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
fn cli(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_luna"));
    command.env("LUNA_SYSROOT", root);
    command
}
fn bounded_output(command: &mut Command, log_prefix: &Path) -> Output {
    // Files avoid pipe backpressure while checking that invalid recursive
    // programs terminate instead of re-enqueuing a failed mono instance.
    let stdout_path = log_prefix.with_extension("stdout");
    let stderr_path = log_prefix.with_extension("stderr");
    let mut child = command
        .stdout(fs::File::create(&stdout_path).unwrap())
        .stderr(fs::File::create(&stderr_path).unwrap())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("CLI did not terminate within 30 seconds: {command:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    Output {
        status,
        stdout: fs::read(stdout_path).unwrap(),
        stderr: fs::read(stderr_path).unwrap(),
    }
}
struct FixtureRoots {
    work: PathBuf,
    source: PathBuf,
    artifact: PathBuf,
}
impl FixtureRoots {
    fn fresh() -> Self {
        let work = std::env::temp_dir().join(format!(
            "luna_literal_parity_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let roots = Self {
            source: work.join("source"),
            artifact: work.join("artifact"),
            work,
        };
        let canonical = workspace().join("libs/external");
        let manifest = SysrootManifest::load_and_validate(&canonical.join("sysroot.toml")).unwrap();
        // Isolate a language regression from unrelated experimental stdlib
        // providers. Use actual canonical bootstrap entries and their dependencies.
        let mut names: Vec<_> = LangContractManifest::canonical()
            .contracts
            .iter()
            .map(|entry| entry.provider_id.to_string())
            .collect();
        names.push("panic".into());
        names.sort();
        names.dedup();
        let mut text = String::new();
        for name in names {
            let entry = manifest.find_provider(&name).unwrap();
            text.push_str(&format!(
                "[[provider]]\nname = {:?}\npath = {:?}\nvisibility = {:?}\n",
                entry.name,
                entry.path,
                if matches!(
                    entry.visibility,
                    luna_driver::sysroot_manifest::ProviderVisibility::Public
                ) {
                    "public"
                } else {
                    "internal"
                }
            ));
            if let Some(contract) = &entry.lang_contract {
                text.push_str(&format!("lang_contract = {contract:?}\n"));
            }
            if !entry.aliases.is_empty() {
                text.push_str(&format!("aliases = {:?}\n", entry.aliases));
            }
            for root in [&roots.source, &roots.artifact] {
                let destination = root
                    .join("libs/external")
                    .join(format!("{}.ln", entry.path));
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::copy(canonical.join(format!("{}.ln", entry.path)), destination).unwrap();
            }
        }
        for root in [&roots.source, &roots.artifact] {
            fs::write(root.join("libs/external/sysroot.toml"), &text).unwrap();
        }
        let build = cli(&roots.artifact)
            .args(["build-sysroot", "--quiet"])
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "fresh bootstrap artifact build: {}",
            render(&build)
        );
        fn remove_sources(root: &Path) {
            for entry in fs::read_dir(root).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    remove_sources(&path);
                } else if path.extension().is_some_and(|ext| ext == "ln") {
                    fs::remove_file(path).unwrap();
                }
            }
        }
        remove_sources(&roots.artifact);
        // The ordinary user provider is built in a fresh compiler process and
        // only its canonical .llib/.obj is copied into the artifact search root.
        let provider = workspace().join("tests/luna/compiler/providers/literal_provider.ln");
        fs::copy(&provider, roots.source.join("literal_provider.ln")).unwrap();
        let build = cli(&roots.artifact)
            .arg("build")
            .arg(&provider)
            .args(["--lib", "--emit", "llib", "--quiet", "-o"])
            .arg(roots.artifact.join("literal_provider.llib"))
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "fresh user provider build: {}",
            render(&build)
        );
        assert!(roots.artifact.join("literal_provider.llib").is_file());
        assert!(roots.artifact.join("literal_provider.obj").is_file());
        assert!(!roots.artifact.join("literal_provider.ln").exists());
        roots
    }
}
impl Drop for FixtureRoots {
    fn drop(&mut self) {
        if self.work.parent() == Some(std::env::temp_dir().as_path())
            && self
                .work
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("luna_literal_parity_")
        {
            let _ = fs::remove_dir_all(&self.work);
        }
    }
}
#[test]
fn literals_full_codegen_and_negative_matrix_match_fresh_artifacts() {
    let roots = FixtureRoots::fresh();
    for fixture in [
        "integer", "bytes", "comptime", "context", "provider", "anchor",
    ] {
        let path = workspace().join(format!(
            "tests/luna/compiler/literal_typing_{fixture}_v1.ln"
        ));
        let mut outcomes = Vec::new();
        for (mode, root) in [("source", &roots.source), ("artifact", &roots.artifact)] {
            let local_fixture = root.join(path.file_name().unwrap());
            fs::copy(&path, &local_fixture).unwrap();
            let executable = roots
                .work
                .join(format!("{fixture}-{mode}"))
                .with_extension(std::env::consts::EXE_EXTENSION);
            let build = cli(root)
                .arg("build")
                .arg(&local_fixture)
                .args(["--quiet", "-I"])
                .arg(root)
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                build.status.success(),
                "{fixture}/{mode}: {}",
                render(&build)
            );
            let run = Command::new(executable).output().unwrap();
            assert!(run.status.success(), "{fixture}/{mode}: {}", render(&run));
            outcomes.push((run.status.code(), run.stdout, run.stderr));
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "{fixture} source/artifact mismatch"
        );
    }
    for (fixture, diagnostic) in [
        ("default_range", "E_INTEGER_LITERAL_RANGE"),
        ("cast_context", "E_INTEGER_LITERAL_RANGE"),
        ("u8_range", "E_INTEGER_LITERAL_RANGE"),
        ("i8_range", "E_INTEGER_LITERAL_RANGE"),
        ("unsigned", "E_INTEGER_LITERAL_RANGE"),
        ("typed_variable", "type mismatch"),
        ("suffix_override", "type mismatch"),
        ("radix", "E_INVALID_INTEGER_LITERAL"),
        ("u128_range", "E_INVALID_INTEGER_LITERAL"),
        ("byte_unicode", "E_INVALID_BYTE_LITERAL"),
        ("byte_escape", "E_INVALID_BYTE_LITERAL"),
        ("byte_length", "type mismatch"),
        ("byte_text", "type mismatch"),
        ("byte_escape_borrow", "LocalBorrowEscape"),
        ("i128_range", "E_INTEGER_LITERAL_RANGE"),
        ("unknown_suffix", "E_INVALID_INTEGER_LITERAL"),
        ("empty_radix", "E_INVALID_INTEGER_LITERAL"),
        ("byte_empty", "E_INVALID_BYTE_LITERAL"),
        ("byte_multiple", "E_INVALID_BYTE_LITERAL"),
        ("comptime_overflow", "overflow"),
        ("pattern_range", "E_INTEGER_LITERAL_RANGE"),
        ("enum_arity", "Enum variant expects 1 fields"),
        ("nonzero_anchor", "RawStorageAnchorMismatch"),
        ("null_anchor_promotion", "RawStorageAnchorViolation"),
        ("recursive_inference", "E_UNCONSTRAINED_INFERENCE"),
    ] {
        let path = workspace().join(format!("tests/luna/compiler/literal_negative_{fixture}.ln"));
        for root in [&roots.source, &roots.artifact] {
            let output = bounded_output(
                cli(root)
                    .arg("build")
                    .arg(&path)
                    .args(["--quiet", "-o"])
                    .arg(roots.work.join("invalid")),
                &roots.work.join(format!("negative-{fixture}")),
            );
            assert_eq!(
                output.status.code(),
                Some(1),
                "negative {fixture}: {}",
                render(&output)
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(diagnostic),
                "negative {fixture}: {}",
                render(&output)
            );
        }
    }
}
