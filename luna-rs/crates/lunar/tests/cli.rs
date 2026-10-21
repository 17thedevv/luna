use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "lunar-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_lunar"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn app(&self, name: &str) -> PathBuf {
        let output = self.cli(&["new", name]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.0.join(name)
    }
    fn check_error(&self, path: &Path, expected: &str) {
        let output = self.cli(&[
            "check",
            "--manifest-path",
            path.join("luna.toml").to_str().unwrap(),
            "--compiler",
            "nonexistent-luna-test-executable",
        ]);
        assert!(!output.status.success());
        let err = String::from_utf8_lossy(&output.stderr);
        assert!(err.contains(expected), "expected {expected}: {err}");
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn creates_shared_manifest_and_does_not_overwrite() {
    let t = Temp::new();
    let path = t.app("hello");
    assert!(path.join("luna.toml").is_file());
    assert!(!path.join("lunar.toml").exists());
    let original = fs::read(path.join("src/main.ln")).unwrap();
    assert!(!t.cli(&["init", "hello"]).status.success());
    assert_eq!(fs::read(path.join("src/main.ln")).unwrap(), original);
}

#[test]
fn init_existing_directory_and_preserves_gitignore() {
    let t = Temp::new();
    fs::write(t.0.join(".gitignore"), "user\n").unwrap();
    assert!(t
        .cli(&["init", ".", "--name", "sample", "--lib"])
        .status
        .success());
    assert!(t.0.join("src/sample.ln").exists());
    assert_eq!(
        fs::read_to_string(t.0.join(".gitignore")).unwrap(),
        "user\n"
    );
}

#[test]
fn rejects_network_dependency_declarations() {
    for dependency in [
        "thing = '1.0'",
        "thing = { git = 'https://example.org/lib' }",
        "thing = { path = 'https://example.org/lib' }",
        "thing = { path = '../local', version = '1.0' }",
    ] {
        let t = Temp::new();
        let path = t.app("app");
        let mut text = fs::read_to_string(path.join("luna.toml")).unwrap();
        text.push_str(&format!("\n[dependencies]\n{dependency}\n"));
        fs::write(path.join("luna.toml"), text).unwrap();
        t.check_error(
            &path,
            if dependency.contains("https://example.org/lib' }") && dependency.contains("path =") {
                "relative path"
            } else if dependency.starts_with("thing = '") {
                "invalid type"
            } else {
                "unknown field"
            },
        );
        assert!(!path.join("target").exists());
    }
}

#[test]
fn rejects_cycle_before_executing_compiler() {
    let t = Temp::new();
    for (a, b) in [("one", "two"), ("two", "one")] {
        assert!(t.cli(&["new", a, "--lib"]).status.success());
        let file = t.0.join(a).join("luna.toml");
        let mut text = fs::read_to_string(&file).unwrap();
        text.push_str(&format!("\n[dependencies]\n{b} = {{ path = '../{b}' }}\n"));
        fs::write(file, text).unwrap();
    }
    t.check_error(&t.0.join("one"), "dependency cycle");
}

#[test]
fn rejects_alias_collision_and_binary_only_dependencies() {
    let t = Temp::new();
    let app = t.app("app");
    let dep = t.app("dep");
    let file = app.join("luna.toml");
    let initial = fs::read_to_string(&file).unwrap();
    fs::write(
        &file,
        format!("{initial}\n[dependencies]\nother = {{ path = '../dep' }}\n"),
    )
    .unwrap();
    t.check_error(&app, "has no [lib]");
    fs::remove_dir_all(dep).unwrap();
    assert!(t.cli(&["new", "dep", "--lib"]).status.success());
    fs::write(file, format!("{initial}\n[dependencies]\nother = {{ path = '../dep' }}\n[providers]\nother = 'missing'\n")).unwrap();
    t.check_error(&app, "conflicting provider binding");
}

#[test]
fn rejects_schema_typos_and_sources_outside_package() {
    let t = Temp::new();
    let app = t.app("app");
    let file = app.join("luna.toml");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, text.replace("schema = 1", "schema = 2")).unwrap();
    t.check_error(&app, "unsupported schema");
    fs::write(&file, text.replace("version =", "versoin =")).unwrap();
    t.check_error(&app, "unknown field");
    fs::write(t.0.join("outside.ln"), "fn main() -> i32 { return 0; }").unwrap();
    fs::write(file, text.replace("src/main.ln", "../outside.ln")).unwrap();
    t.check_error(&app, "inside its package");
}

#[test]
fn clean_requires_ownership_and_preserves_other_targets() {
    let t = Temp::new();
    let app = t.app("app");
    fs::create_dir_all(app.join("target/lunar")).unwrap();
    fs::write(app.join("target/lunar/user.txt"), "keep").unwrap();
    let args = [
        "clean",
        "--manifest-path",
        app.join("luna.toml").to_str().unwrap(),
    ]
    .map(String::from);
    let output = t.cli(&args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(!output.status.success());
    assert!(app.join("target/lunar/user.txt").exists());
    fs::remove_dir_all(app.join("target/lunar")).unwrap();
    fs::write(app.join("target/user.txt"), "keep").unwrap();
    assert!(t
        .cli(&args.iter().map(String::as_str).collect::<Vec<_>>())
        .status
        .success());
    assert!(app.join("target/user.txt").exists());
    assert!(!app.join("target/lunar").exists());
}

#[test]
fn no_tests_and_ambiguous_run_reject_without_compiler() {
    let t = Temp::new();
    let app = t.app("app");
    let manifest = app.join("luna.toml");
    let path = manifest.to_str().unwrap();
    let output = t.cli(&["test", "--manifest-path", path]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("no [[test]]"));
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str("\n[[bin]]\nname='second'\npath='src/main.ln'\n");
    fs::write(&manifest, text).unwrap();
    let output = t.cli(&["run", "--manifest-path", path]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("select one"));
}

#[test]
fn lock_prevents_clean_during_build() {
    let t = Temp::new();
    let app = t.app("app");
    fs::create_dir(app.join("target")).unwrap();
    let lock = fs::File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(app.join("target/.lunar.lock"))
        .unwrap();
    lock.lock().unwrap();
    let output = t.cli(&[
        "clean",
        "--manifest-path",
        app.join("luna.toml").to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("another lunar process"));
}

#[cfg(unix)]
#[test]
fn clean_rejects_symlinked_target() {
    let t = Temp::new();
    let app = t.app("app");
    let outside = t.0.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), "keep").unwrap();
    std::os::unix::fs::symlink(&outside, app.join("target")).unwrap();
    let output = t.cli(&[
        "clean",
        "--manifest-path",
        app.join("luna.toml").to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(outside.join("keep").exists());
}

#[cfg(unix)]
#[test]
fn init_rejects_dangling_manifest_and_external_source_directory() {
    use std::os::unix::fs::symlink;
    let t = Temp::new();
    symlink("missing", t.0.join("luna.toml")).unwrap();
    let output = t.cli(&["init", ".", "--name", "app"]);
    assert!(!output.status.success());
    assert!(!t.0.join("missing").exists());
    fs::remove_file(t.0.join("luna.toml")).unwrap();
    let outside = Temp::new();
    symlink(&outside.0, t.0.join("src")).unwrap();
    let output = t.cli(&["init", ".", "--name", "app"]);
    assert!(!output.status.success());
    assert!(!outside.0.join("main.ln").exists());
}

#[cfg(windows)]
#[test]
fn clean_rejects_junction_target() {
    let t = Temp::new();
    let app = t.app("app");
    let outside = t.0.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), "keep").unwrap();
    let junction = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(app.join("target"))
        .arg(&outside)
        .output()
        .unwrap();
    assert!(
        junction.status.success(),
        "{}",
        String::from_utf8_lossy(&junction.stderr)
    );
    let output = t.cli(&[
        "clean",
        "--manifest-path",
        app.join("luna.toml").to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("reparse"));
    assert!(outside.join("keep").exists());
    fs::remove_dir(app.join("target")).unwrap();
}
