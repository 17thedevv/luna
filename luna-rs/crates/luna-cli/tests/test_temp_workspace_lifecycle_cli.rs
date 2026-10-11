//! R5-HARNESS-01 freeze — test-infrastructure ownership and lifecycle.
//!
//! `ProviderModes::fresh()` must own its temporary sysroot through RAII:
//! sequential creation must not accumulate directories, a panic unwind must
//! still remove them, and the process-scoped temp root used by suite-local
//! helpers must be isolated and guarded.
#[path = "support/stdlib.rs"]
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use support::ProviderModes;

const WORKSPACE_PREFIX: &str = "luna_test_ws_";

fn fixture(name: &str) -> PathBuf {
    support::workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/comptime_diagnostic_cascade")
        .join(format!("{name}.ln"))
}

#[test]
fn sequential_fresh_provider_modes_do_not_accumulate() {
    let mut workspaces = Vec::new();
    for _ in 0..2 {
        let modes = ProviderModes::fresh();
        let work = modes.source.parent().expect("provider work dir").to_path_buf();
        assert!(work.is_dir(), "workspace must exist while owned: {}", work.display());
        workspaces.push(work);
        drop(modes);
    }
    for work in &workspaces {
        assert!(
            !work.exists(),
            "RAII owner must remove its sysroot on drop: {}",
            work.display()
        );
    }
    // Other tests run in parallel inside this binary, so the process-wide count
    // is not a stable signal; what must hold is that every workspace this test
    // created is gone.
    assert!(
        workspaces.iter().all(|work| !work.exists()),
        "sequential creation left temp workspaces behind"
    );
}

#[test]
fn a_dropped_provider_modes_leaves_no_generated_sysroot() {
    let modes = ProviderModes::fresh();
    let work = modes.source.parent().unwrap().to_path_buf();
    let source = modes.source.clone();
    assert!(source.join("libs/external").is_dir());
    drop(modes);
    assert!(!work.exists(), "generated sysroot survived: {}", work.display());
}

#[test]
fn panic_unwind_removes_the_temp_sysroot() {
    let seen: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    let capture = Arc::clone(&seen);
    let result = std::panic::catch_unwind(move || {
        let modes = ProviderModes::fresh();
        *capture.lock().unwrap() = modes.source.parent().map(Path::to_path_buf);
        panic!("simulate a failing acceptance test after the sysroot exists");
    });
    assert!(result.is_err(), "the simulated test must fail");
    let work = seen.lock().unwrap().clone().expect("workspace path captured");
    assert!(
        !work.exists(),
        "panic unwind must still clean up the temp sysroot: {}",
        work.display()
    );
}

#[test]
fn process_temp_root_is_owned_and_unique_per_call() {
    let first = luna_test_support::owned_temp_root();
    let second = luna_test_support::owned_temp_root();
    assert_ne!(
        first, second,
        "each call must get its own directory so parallel helpers cannot share one"
    );
    for path in [&first, &second] {
        assert!(path.is_dir(), "the owned root must exist: {}", path.display());
        let process_root = path
            .ancestors()
            .find(|ancestor| {
                ancestor
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(WORKSPACE_PREFIX))
            })
            .expect("owned root must sit under a guarded process root");
        assert_eq!(
            process_root.parent(),
            Some(std::env::temp_dir().as_path()),
            "the process root must live directly under the temp dir"
        );
    }
}

#[test]
fn parallel_independent_workspaces_are_distinct() {
    let handles: Vec<_> = (0..3)
        .map(|_| {
            std::thread::spawn(|| {
                let modes = ProviderModes::fresh();
                let work = modes.source.parent().unwrap().to_path_buf();
                assert!(work.is_dir());
                work
            })
        })
        .collect();
    let workspaces: Vec<PathBuf> = handles
        .into_iter()
        .map(|handle| handle.join().expect("worker test thread"))
        .collect();
    for (index, work) in workspaces.iter().enumerate() {
        for other in workspaces.iter().skip(index + 1) {
            assert_ne!(work, other, "parallel workspaces must not share a directory");
        }
        assert!(!work.exists(), "worker workspace must be removed: {}", work.display());
    }
}

#[test]
fn a_failing_check_still_leaves_the_workspace_removable() {
    // Exercising a real failing compile keeps the cleanup path on the busy path,
    // not only on the happy path.
    let modes = ProviderModes::fresh();
    let work = modes.source.parent().unwrap().to_path_buf();
    let output = modes.check(&modes.source, &fixture("semantic_inside_comptime"));
    assert_eq!(output.status.code(), Some(1), "fixture must fail to compile");
    drop(modes);
    assert!(!work.exists(), "workspace survived a failing compile: {}", work.display());
}
