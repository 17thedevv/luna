use luna_driver::sysroot::Sysroot;
use luna_driver::{compile, CompilerOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "luna_generic_trait_method_{}_{}_{}",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create generic trait method test directory");
    dir
}

fn compile_fixture(name: &str, source: &str, search_paths: &[String]) -> Result<PathBuf, String> {
    let dir = temp_dir(name);
    compile_fixture_in(&dir, source, search_paths)
}

fn compile_fixture_in(dir: &Path, source: &str, search_paths: &[String]) -> Result<PathBuf, String> {
    let source_path = dir.join("main.ln");
    let executable = dir.join(if cfg!(windows) { "main.exe" } else { "main" });
    fs::write(&source_path, source).expect("write Luna fixture");
    let options = CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        search_paths: search_paths.to_vec(),
        quiet: true,
        ..Default::default()
    };
    compile(&source_path.to_string_lossy(), source.to_string(), &options)
        .map(|_| executable)
        .map_err(|diagnostics| format!("{diagnostics:#?}"))
}

fn execute(executable: &Path) -> (i32, String, String) {
    let output = Command::new(executable).output().expect("execute Luna fixture");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn source_sysroot() -> PathBuf {
    Sysroot::discover_for_test()
        .expect("canonical sysroot must exist")
        .root()
        .to_path_buf()
}

#[test]
fn generic_trait_methods_monomorphize_for_concrete_and_generic_callers() {
    let fixture = include_str!("../../../tests/luna/language/generic_trait_method_monomorphization.ln");
    let sysroot = source_sysroot();
    let exe = compile_fixture(
        "source_codegen",
        fixture,
        &[sysroot.to_string_lossy().into_owned()],
    )
    .unwrap_or_else(|diagnostics| panic!("generic trait-method fixture must fully codegen: {diagnostics}"));
    let result = execute(&exe);
    assert_eq!(result.0, 0, "generic trait-method behavior failed: {result:?}");
}

#[test]
fn generic_trait_method_bound_misuse_is_rejected_at_compile_time() {
    let fixture = include_str!("../../../tests/luna/language/generic_trait_method_bad_bound.ln");
    let sysroot = source_sysroot();
    let error = compile_fixture(
        "bad_method_bound",
        fixture,
        &[sysroot.to_string_lossy().into_owned()],
    )
    .expect_err("u64 must not satisfy a method-level Supported bound");
    assert!(
        error.contains("TRAIT_BOUND") || error.contains("trait bound") || error.contains("does not implement"),
        "expected a deterministic method-level trait-bound diagnostic, got: {error}"
    );
}

#[test]
fn generic_trait_method_uninferable_argument_is_rejected() {
    let fixture = include_str!("../../../tests/luna/language/generic_trait_method_uninferable.ln");
    let sysroot = source_sysroot();
    let error = compile_fixture(
        "uninferable_method_type",
        fixture,
        &[sysroot.to_string_lossy().into_owned()],
    )
    .expect_err("method type parameter absent from inputs/expected type must be rejected");
    assert!(
        error.contains("E_UNCONSTRAINED_INFERENCE") || error.contains("inference"),
        "expected a deterministic inference diagnostic, got: {error}"
    );
}

#[test]
fn generic_mutable_trait_method_keeps_receiver_mutability_rules() {
    let fixture = include_str!("../../../tests/luna/language/generic_trait_method_shared_mutable_rejected.ln");
    let sysroot = source_sysroot();
    let error = compile_fixture(
        "shared_mutable_receiver",
        fixture,
        &[sysroot.to_string_lossy().into_owned()],
    )
    .expect_err("generic method requiring &rw self must reject an immutable receiver");
    assert!(
        error.contains("E_CANNOT_MUTATE_IMMUTABLE_POINTER"),
        "expected the ordinary mutable-receiver diagnostic, got: {error}"
    );
}

#[test]
fn generic_trait_method_source_and_fresh_llib_only_provider_match() {
    let provider = include_str!("../../../tests/luna/language/generic_trait_method_provider.ln");
    let consumer = include_str!("../../../tests/luna/language/generic_trait_method_provider_consumer.ln");
    let sysroot = source_sysroot();

    let source_dir = temp_dir("provider_source");
    let source_path = source_dir.join("generic_trait_method_provider.ln");
    fs::write(&source_path, provider).expect("write source provider");
    let source_exe = compile_fixture_in(
        &source_dir,
        consumer,
        &[
            source_dir.to_string_lossy().into_owned(),
            sysroot.to_string_lossy().into_owned(),
        ],
    )
    .unwrap_or_else(|diagnostics| panic!("source provider consumer must codegen: {diagnostics}"));
    let source_result = execute(&source_exe);

    let artifact_dir = temp_dir("provider_artifact");
    let provider_path = artifact_dir.join("generic_trait_method_provider.ln");
    let artifact_path = artifact_dir.join("generic_trait_method_provider.llib");
    fs::write(&provider_path, provider).expect("write artifact provider source");
    let build_options = CompilerOptions {
        output_path: Some(artifact_path.to_string_lossy().into_owned()),
        search_paths: vec![sysroot.to_string_lossy().into_owned()],
        emit_llib: true,
        no_link: true,
        quiet: true,
        ..Default::default()
    };
    compile(&provider_path.to_string_lossy(), provider.to_string(), &build_options)
        .unwrap_or_else(|diagnostics| panic!("fresh provider .llib build failed: {diagnostics:#?}"));
    assert!(artifact_path.exists(), "fresh provider .llib must be written");
    fs::remove_file(&provider_path).expect("remove provider source to prohibit fallback");
    assert!(
        !provider_path.exists(),
        "artifact-only test must not have a provider .ln fallback"
    );

    let artifact_exe = compile_fixture_in(
        &artifact_dir,
        consumer,
        &[
            artifact_dir.to_string_lossy().into_owned(),
            sysroot.to_string_lossy().into_owned(),
        ],
    )
    .unwrap_or_else(|diagnostics| panic!("artifact-only provider consumer must codegen: {diagnostics}"));
    let artifact_result = execute(&artifact_exe);

    assert_eq!(source_result.0, 0, "source provider behavior failed: {source_result:?}");
    assert_eq!(artifact_result.0, 0, "artifact provider behavior failed: {artifact_result:?}");
    assert_eq!(source_result, artifact_result, "source and fresh .llib-only semantics differ");
}
