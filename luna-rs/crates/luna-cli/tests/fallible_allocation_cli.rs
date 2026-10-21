#[path = "support/stdlib.rs"]
mod support;

use luna_test_support::TempWorkspace;
use std::path::PathBuf;
use std::process::Command;
use support::{ProviderModes, render, workspace_root};

fn failure_runtime(work: &TempWorkspace) -> PathBuf {
    if let Some(path) = std::env::var_os("LUNA_ALLOC_TEST_RUNTIME_LIB") {
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "test runtime does not exist: {}",
            path.display()
        );
        return path;
    }
    let build = work.join("runtime");
    let mut configure = Command::new("cmake");
    if cfg!(windows) {
        // The supported Windows runtime is GNU. CC may name Luna's linker
        // wrapper; that executable is not a CMake compiler driver.
        configure.args(["-DCMAKE_C_COMPILER=gcc", "-DCMAKE_CXX_COMPILER=g++"]);
    }
    let configured = configure
        .arg("-S")
        .arg(workspace_root().parent().unwrap().join("runtime"))
        .arg("-B")
        .arg(&build)
        .args(["-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release"])
        .output()
        .expect("CMake is required for deterministic runtime failure tests");
    assert!(configured.status.success(), "{}", render(&configured));
    let built = Command::new("cmake")
        .arg("--build")
        .arg(&build)
        .args(["--target", "luna-runtime-alloc-test", "--parallel", "2"])
        .output()
        .unwrap();
    assert!(built.status.success(), "{}", render(&built));
    [
        "libluna-runtime-alloc-test.a",
        "luna-runtime-alloc-test.lib",
    ]
    .into_iter()
    .map(|name| build.join(name))
    .find(|path| path.is_file())
    .expect("test runtime archive")
}

#[test]
fn allocation_success_and_failure_preserve_ownership_in_both_provider_modes() {
    let runtime_work = TempWorkspace::new("allocation_runtime");
    let runtime = failure_runtime(&runtime_work);
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/stdlib/fallible_allocation");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let source = fixtures.join("success.ln");
        let check = modes.check(root, &source);
        assert!(check.status.success(), "{mode}: {}", render(&check));
        let (exe, build) = modes.build(root, &source, &format!("success_{mode}"));
        assert!(build.status.success(), "{mode}: {}", render(&build));
        let run = Command::new(exe).output().unwrap();
        assert!(run.status.success(), "{mode}: {}", render(&run));
        for name in ["failure_ownership", "borrowed_values"] {
            let source = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &source);
            assert!(check.status.success(), "{mode}/failure: {}", render(&check));
            let (exe, build) =
                modes.build_with_runtime(root, &source, &format!("{name}_{mode}"), Some(&runtime));
            assert!(build.status.success(), "{mode}/failure: {}", render(&build));
            let run = Command::new(exe).output().unwrap();
            assert!(run.status.success(), "{mode}/failure: {}", render(&run));
        }
        for (name, code) in [
            ("abort_ordinary", "PANIC [L001]"),
            ("abort_capacity_overflow", "PANIC [L005]"),
        ] {
            let source = fixtures.join(format!("{name}.ln"));
            let (exe, build) =
                modes.build_with_runtime(root, &source, &format!("{name}_{mode}"), Some(&runtime));
            assert!(build.status.success(), "{mode}/abort: {}", render(&build));
            let run = Command::new(exe).output().unwrap();
            assert!(!run.status.success());
            assert!(
                String::from_utf8_lossy(&run.stderr).contains(code),
                "{}",
                render(&run)
            );
        }
        for (name, code) in [
            ("reject_borrowed_mutation", "error[E3003]"),
            ("reject_moved_input", "error[E3001]"),
            ("reject_local_escape", "error[E3005]"),
            ("reject_error_borrow_mutation", "error[E3003]"),
        ] {
            let source = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &source);
            let (exe, build) = modes.build(root, &source, &format!("{name}_{mode}"));
            assert!(!exe.exists());
            for result in [check, build] {
                assert!(
                    !result.status.success(),
                    "{mode}/{name}: {}",
                    render(&result)
                );
                assert!(
                    String::from_utf8_lossy(&result.stderr).contains(code),
                    "{mode}/{name}: {}",
                    render(&result)
                );
            }
        }
    }
}
