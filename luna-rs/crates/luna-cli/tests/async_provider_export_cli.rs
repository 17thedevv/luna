//! FIND-ASYNC-PROVIDER-01 — freeze the provider/artifact reducer that exposed
//! the MVIR HeapFree wire-tag drift. Async runtime semantics are covered elsewhere.
#[path = "support/stdlib.rs"]
mod support;

use std::{fs, path::{Path, PathBuf}, process::Command};
use support::{render, ProviderModes};

struct Case {
    name: &'static str,
    provider: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "async_zero_i32",
        provider: "export module m { export async fn f() -> i32 { return 10; } }\n",
    },
    Case {
        name: "async_one_i32",
        provider: "export module m { export async fn f(x: i32) -> i32 { return x; } }\n",
    },
    Case {
        name: "sync_zero_i32",
        provider: "export module m { export fn f() -> i32 { return 10; } }\n",
    },
    Case {
        name: "async_generic_one",
        provider: "export module m { export async fn f<T>(x: T) -> T { return x; } }\n",
    },
    Case {
        name: "async_zero_void",
        provider: "export module m { export async fn f() {} }\n",
    },
];

fn publish(root: &Path, dir: &Path, source: &str) {
    fs::create_dir_all(dir).unwrap();
    let provider = dir.join("provider.ln");
    fs::write(&provider, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(dir.join("provider.llib"))
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "provider publish: {}", render(&output));
}

fn consumer(dir: &Path) -> PathBuf {
    let input = dir.join("main.ln");
    fs::write(
        &input,
        "import \"provider\";\nfn main() -> i32 { return 0; }\n",
    )
    .unwrap();
    input
}

fn assert_builds(modes: &ProviderModes, root: &Path, dir: &Path, tag: &str) {
    let input = consumer(dir);
    let (_, output) = modes.build(root, &input, tag);
    assert!(output.status.success(), "{tag}: {}", render(&output));
}

#[test]
fn async_provider_export_control_matrix_matches_source_and_fresh_artifacts() {
    let modes = ProviderModes::fresh();

    for case in CASES {
        let source_dir = modes.source.join(format!("find_async_provider_source_{}", case.name));
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("provider.ln"), case.provider).unwrap();
        assert_builds(
            &modes,
            &modes.source,
            &source_dir,
            &format!("find_async_source_{}", case.name),
        );

        let artifact_dir = modes.artifact.join(format!("find_async_provider_artifact_{}", case.name));
        publish(&modes.artifact, &artifact_dir, case.provider);
        fs::remove_file(artifact_dir.join("provider.ln")).unwrap();
        assert_builds(
            &modes,
            &modes.artifact,
            &artifact_dir,
            &format!("find_async_artifact_{}", case.name),
        );
    }
}

#[test]
fn zero_parameter_async_fresh_artifact_survives_relocation() {
    let modes = ProviderModes::fresh();
    let original = modes.artifact.join("find_async_provider_relocate_original");
    publish(&modes.artifact, &original, CASES[0].provider);
    fs::remove_file(original.join("provider.ln")).unwrap();
    consumer(&original);

    let relocated = modes.artifact.join("find_async_provider_relocate_moved");
    fs::rename(&original, &relocated).unwrap();
    assert_builds(
        &modes,
        &modes.artifact,
        &relocated,
        "find_async_artifact_relocated",
    );
}
