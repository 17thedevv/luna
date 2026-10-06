//! Compiler-level evidence that a consuming closure invocation tears down its
//! environment: every exit path frees the heap environment, and a capture that
//! is still initialized on a path is dropped before the environment is freed.
#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn consuming_closure_frees_environment_on_every_exit_path() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/generic_drop");
    let root = &modes.source;
    let project = root.join("mvir_teardown");
    fs::create_dir(&project).unwrap();
    fs::copy(fixtures.join("resource.ln"), project.join("resource.ln")).unwrap();
    let src = project.join("closure_env_teardown_mvir.ln");
    fs::copy(fixtures.join("closure_env_teardown_mvir.ln"), &src).unwrap();
    let exe = project.join("teardown.exe");
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&src)
        .arg("-o")
        .arg(&exe)
        .arg("--emit")
        .arg("mvir")
        .arg("--quiet")
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", render(&output));
    let mvir = fs::read_to_string(exe.with_extension("mvir")).unwrap();

    // Isolate the consuming closure function by its canonical name.
    let chunk = mvir
        .split("\n        Function {")
        .find(|chunk| {
            let head: String = chunk.chars().take(200).collect();
            head.contains("name: GlobalId") && head.contains("name: \"closure_")
        })
        .expect("consuming closure function present in MVIR");

    let heap_free = chunk.matches("HeapFree").count();
    let rets = chunk.matches("Ret {").count();
    let drops = chunk.matches("Drop {").count();

    assert!(
        rets >= 2,
        "expected multiple closure exit paths, got {rets}\n{chunk}"
    );
    assert!(
        heap_free >= rets,
        "every closure exit path must free the environment: heap_free={heap_free} rets={rets}\n{chunk}"
    );
    assert!(
        drops >= 1,
        "an initialized capture must be dropped before the environment is freed\n{chunk}"
    );
}
