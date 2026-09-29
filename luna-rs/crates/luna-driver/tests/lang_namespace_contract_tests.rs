use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{check_semantic_only, compile, CompilerOptions};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

static SOURCE_ONLY_SYSROOT: OnceLock<PathBuf> = OnceLock::new();

fn source_only_sysroot_root() -> &'static PathBuf {
    SOURCE_ONLY_SYSROOT.get_or_init(|| {
        let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("libs/external");
        let root = temp_sysroot("shared_source_only");
        copy_tree(&canonical_external, &root.join("libs/external"));
        remove_artifacts_recursively(&root);
        root
    })
}

fn compile_source(name: &str, source: &str) -> Result<(), String> {
    let root = source_only_sysroot_root();
    let dir = std::env::temp_dir().join(format!("luna_std_namespace_{name}"));
    fs::create_dir_all(&dir).expect("create temporary fixture directory");
    let source_path = dir.join("main.ln");
    fs::write(&source_path, source).expect("write temporary Luna source");
    let options = CompilerOptions {
        output_path: Some(dir.join("main.obj").to_string_lossy().into_owned()),
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        no_link: true,
        ..Default::default()
    };
    compile(&source_path.to_string_lossy(), source.to_string(), &options)
        .map(|_| ())
        .map_err(|diagnostics| format!("{diagnostics:#?}"))
}

fn check_source_semantics(name: &str, source: &str) -> Result<(), String> {
    check_source_semantics_at(name, source, source_only_sysroot_root())
}

fn check_source_semantics_at(name: &str, source: &str, root: &Path) -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("luna_std_namespace_{name}"));
    fs::create_dir_all(&dir).expect("create temporary fixture directory");
    let source_path = dir.join("main.ln");
    fs::write(&source_path, source).expect("write temporary Luna source");
    let options = CompilerOptions {
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        ..Default::default()
    };
    check_semantic_only(&source_path.to_string_lossy(), source.to_string(), &options)
        .map_err(|diagnostics| format!("{diagnostics:#?}"))
}

fn compile_and_run_source_at(name: &str, source: &str, root: &Path) -> Result<i32, String> {
    let dir = std::env::temp_dir().join(format!("luna_std_namespace_{name}"));
    fs::create_dir_all(&dir).expect("create temporary fixture directory");
    let source_path = dir.join("main.ln");
    fs::write(&source_path, source).expect("write temporary Luna source");
    let executable = dir.join("main.exe");
    let options = CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        no_link: false,
        ..Default::default()
    };
    compile(&source_path.to_string_lossy(), source.to_string(), &options)
        .map_err(|diagnostics| format!("{diagnostics:#?}"))?;
    let status = Command::new(&executable)
        .status()
        .map_err(|error| format!("could not execute {}: {error}", executable.display()))?;
    status
        .code()
        .ok_or_else(|| "test executable terminated without an exit code".to_string())
}

fn compile_and_run_output_at(
    name: &str,
    source: &str,
    root: &Path,
) -> Result<(i32, String, String), String> {
    let dir = std::env::temp_dir().join(format!("luna_std_namespace_{name}"));
    fs::create_dir_all(&dir).expect("create temporary fixture directory");
    let source_path = dir.join("main.ln");
    fs::write(&source_path, source).expect("write temporary Luna source");
    let executable = dir.join("main.exe");
    let options = CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        no_link: false,
        ..Default::default()
    };
    compile(&source_path.to_string_lossy(), source.to_string(), &options)
        .map_err(|diagnostics| format!("{diagnostics:#?}"))?;
    let output = Command::new(&executable)
        .output()
        .map_err(|error| format!("could not execute {}: {error}", executable.display()))?;
    Ok((
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create isolated sysroot directory");
    for entry in fs::read_dir(source).expect("enumerate sysroot directory") {
        let entry = entry.expect("read sysroot directory entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy sysroot file");
        }
    }
}

fn remove_artifacts_recursively(root: &Path) {
    for entry in fs::read_dir(root).expect("enumerate isolated sysroot") {
        let path = entry.expect("read isolated sysroot entry").path();
        if path.is_dir() {
            remove_artifacts_recursively(&path);
        } else if matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("llib" | "obj")
        ) {
            fs::remove_file(path).expect("remove artifact for source-only parity mode");
        }
    }
}

fn remove_sources_recursively(root: &Path) {
    for entry in fs::read_dir(root).expect("enumerate isolated sysroot") {
        let path = entry.expect("read isolated sysroot entry").path();
        if path.is_dir() {
            remove_sources_recursively(&path);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("ln") {
            fs::remove_file(path).expect("remove source for artifact-only parity mode");
        }
    }
}

fn temp_sysroot(name: &str) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "luna_std_namespace_{name}_{}_{}",
        std::process::id(),
        suffix
    ))
}

#[test]
fn canonical_std_language_contract_paths_compile() {
    let source = r#"
fn make_option() -> std::Option<i32> {
    return std::Option::Some(42);
}

fn consume_iterator<T: std::Iterator<i32>>(iter: &rw T) {
    dec item = iter.next();
}

struct CopyProbe { value: i32 };
impl std::Copy for CopyProbe {}
fn require_copy<T: std::Copy>(value: T) {}

struct DropProbe { value: i32 };
impl std::Drop for DropProbe {
    fn drop(self: &rw Self) {}
}

fn main() {
    require_copy(CopyProbe { value: 1 });
    dec probe = DropProbe { value: 1 };
}
"#;
    compile_source("canonical", source)
        .unwrap_or_else(|diagnostics| panic!("canonical std paths must resolve: {diagnostics}"));
}

#[test]
fn source_only_vec_provider_smoke() {
    compile_source(
        "vec_source_smoke",
        "import <copy>; import <vec>; fn needs_copy<T: std::Copy>(value: T) {} fn main() { needs_copy(1 as i32); }",
    )
        .unwrap_or_else(|diagnostics| panic!("Vec provider source must typecheck: {diagnostics}"));
}

#[test]
fn question_mark_uses_canonical_try_contract_semantics() {
    let source = r#"
import <result>;
fn use_try(value: std::Result<i32, i32>) -> std::Result<i32, i32> {
    dec item = value?;
    return std::Result::Ok(item);
}
fn main() {}
"#;
    check_source_semantics("try_semantics", source)
        .unwrap_or_else(|diagnostics| panic!("`?` must resolve through std::Try: {diagnostics}"));
}

#[test]
fn question_mark_full_codegen_matches_source_and_fresh_artifact_only_sysroots() {
    let fixture = include_str!("../../../tests/luna/language/try_codegen_canonical_std_try.ln");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");
    let artifact_root = temp_sysroot("try_codegen_artifact");
    copy_tree(&canonical_external, &artifact_root.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("artifact sysroot"))
        .build_all(true)
        .expect("fresh canonical artifacts must build from current provider sources");

    let source_root = temp_sysroot("try_codegen_source_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("try_codegen_artifact_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);

    let source_result = compile_and_run_source_at("try_codegen_source_only", fixture, &source_root);
    let artifact_result =
        compile_and_run_source_at("try_codegen_artifact_only", fixture, &artifact_only_root);
    assert_eq!(
        source_result,
        Ok(0),
        "source-only ? program should execute successfully"
    );
    assert_eq!(
        artifact_result,
        Ok(0),
        "fresh artifact-only ? program should execute successfully"
    );
    assert_eq!(
        source_result, artifact_result,
        "source and artifact ? behavior must agree"
    );
}

#[test]
fn generic_question_mark_codegen_matches_source_and_fresh_artifact_only_sysroots() {
    let fixture = include_str!("../../../tests/luna/language/try_codegen_generic_cross_module_std_try.ln");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");
    let artifact_root = temp_sysroot("generic_try_codegen_artifact");
    copy_tree(&canonical_external, &artifact_root.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("artifact sysroot"))
        .build_all(true)
        .expect("fresh canonical artifacts must build from current provider sources");

    let source_root = temp_sysroot("generic_try_codegen_source_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("generic_try_codegen_artifact_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);

    let source_result = compile_and_run_source_at(
        "generic_try_codegen_source_only",
        fixture,
        &source_root,
    );
    let artifact_result = compile_and_run_source_at(
        "generic_try_codegen_artifact_only",
        fixture,
        &artifact_only_root,
    );
    assert_eq!(
        source_result,
        Ok(0),
        "generic ? source-only program should execute successfully"
    );
    assert_eq!(
        artifact_result,
        Ok(0),
        "generic ? fresh artifact-only program should execute successfully"
    );
    assert_eq!(
        source_result, artifact_result,
        "generic ? source and artifact behavior must agree"
    );
}

#[test]
fn canonical_language_contracts_match_fresh_artifact_and_source_only_sysroots() {
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");
    let artifact_root = temp_sysroot("artifact");
    let artifact_external = artifact_root.join("libs/external");
    copy_tree(&canonical_external, &artifact_external);
    SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("artifact sysroot"))
        .build_all(true)
        .expect("fresh canonical artifacts must build from current provider sources");

    let source_root = temp_sysroot("source_only");
    copy_tree(&artifact_external, &source_root.join("libs/external"));
    remove_artifacts_recursively(&source_root);

    let valid = r#"
import <result>;
fn make_option() -> std::Option<i32> { return std::Option::Some(42); }
fn consume<T: std::Iterator<i32>>(iter: &rw T) { dec item = iter.next(); }
struct CopyProbe { value: i32 };
impl std::Copy for CopyProbe {}
fn require_copy<T: std::Copy>(value: T) {}
struct DropProbe { value: i32 };
impl std::Drop for DropProbe { fn drop(self: &rw Self) {} }
fn use_try(value: std::Result<i32, i32>) -> std::Result<i32, i32> {
    dec item = value?;
    return std::Result::Ok(item);
}
fn main() {}
"#;
    let artifact_valid = check_source_semantics_at("fresh_artifact_valid", valid, &artifact_root);
    let source_valid = check_source_semantics_at("source_only_valid", valid, &source_root);
    assert!(
        artifact_valid.is_ok(),
        "fresh artifact sysroot rejected canonical paths: {artifact_valid:?}"
    );
    assert!(
        source_valid.is_ok(),
        "source-only sysroot rejected canonical paths: {source_valid:?}"
    );

    let invalid = r#"
fn make_option() -> Option<i32> { return Option::Some(42); }
fn main() {}
"#;
    let artifact_invalid =
        check_source_semantics_at("fresh_artifact_invalid", invalid, &artifact_root);
    let source_invalid = check_source_semantics_at("source_only_invalid", invalid, &source_root);
    assert!(
        artifact_invalid.is_err(),
        "fresh artifact sysroot retained a bare Option alias"
    );
    assert!(
        source_invalid.is_err(),
        "source-only sysroot retained a bare Option alias"
    );

    // The same unresolved-path fallback used to let an explicit impl header
    // with an unknown trait path masquerade as an inherent impl. Recheck the
    // other language-item names in that precise context in both artifact modes.
    for (index, trait_name) in ["Option", "Iterator", "IntoIterator", "Try", "UnknownTrait"]
        .into_iter()
        .enumerate()
    {
        let source = format!(
            "struct Probe {{}}; impl {trait_name} for Probe {{}} fn main() {{}}"
        );
        for (mode, root) in [("artifact", &artifact_root), ("source", &source_root)] {
            let result = check_source_semantics_at(
                &format!("bare_impl_path_{index}_{mode}"),
                &source,
                root,
            );
            let diagnostics = match result {
                Err(diagnostics) => diagnostics,
                Ok(()) => panic!("bare impl path `{trait_name}` unexpectedly resolved in {mode} mode"),
            };
            assert!(
                diagnostics.contains("cannot resolve trait path"),
                "`impl {trait_name}` must fail specifically during path resolution in {mode} mode, got: {diagnostics}"
            );
        }
    }
}

#[test]
fn compiler_language_contracts_do_not_create_root_aliases() {
    let bare_option = r#"
fn make_option() -> Option<i32> {
    return Option::Some(42);
}
fn main() {}
"#;
    assert!(
        compile_source("bare_option", bare_option).is_err(),
        "bare Option must not remain a public root alias"
    );

    let bare_iterator = r#"
fn consume_iterator<T: Iterator<i32>>(iter: &rw T) {
    dec item = iter.next();
}
fn main() {}
"#;
    assert!(
        compile_source("bare_iterator", bare_iterator).is_err(),
        "bare Iterator must not remain a public root alias"
    );

    let bare_try = r#"
fn requires_try<T: Try>(value: T) {
    dec flow = value.branch();
}
fn main() {}
"#;
    let bare_try_result = compile_source("bare_try", bare_try);
    assert!(
        bare_try_result
            .as_ref()
            .is_err_and(|diagnostics| diagnostics.contains("Method `branch` not found")),
        "a bare Try path must not provide trait method visibility: {bare_try_result:?}"
    );
}

#[test]
fn ordinary_core_surface_is_canonical_and_matches_fresh_artifacts() {
    let fixture = include_str!("../../../tests/luna/stdlib/core/ordinary_core_surface.ln");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");
    let artifact_root = temp_sysroot("ordinary_core_artifact");
    copy_tree(&canonical_external, &artifact_root.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("artifact sysroot"))
        .build_all(true)
        .expect("fresh canonical artifacts must build from current provider sources");

    let source_root = temp_sysroot("ordinary_core_source_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("ordinary_core_artifact_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);

    let source_result =
        compile_and_run_source_at("ordinary_core_source_only", fixture, &source_root);
    let artifact_result =
        compile_and_run_source_at("ordinary_core_artifact_only", fixture, &artifact_only_root);
    assert_eq!(
        source_result,
        Ok(0),
        "canonical ordinary core source surface should execute: {source_result:?}"
    );
    assert_eq!(
        artifact_result,
        Ok(0),
        "canonical ordinary core artifact surface should execute: {artifact_result:?}"
    );
    assert_eq!(
        source_result, artifact_result,
        "ordinary core source and artifact behavior must agree"
    );

    for (name, source) in [
        (
            "bare_result",
            "import <result>; fn main() { dec r: Result<i32, i32> = Result::Ok(1); }",
        ),
        (
            "bare_convert",
            "import <convert>; fn main() { dec x: i32 = 1.convert(); }",
        ),
        (
            "bare_default",
            "import <default>; fn main() { dec x: i32 = Default::default(); }",
        ),
        (
            "provider_convert",
            "import <convert>; fn widen<T: convert::Convert<i64>>(x: T) -> i64 { return x.convert(); } fn main() { dec y = widen(1 as u8); }",
        ),
        (
            "provider_ord",
            "import <cmp>; fn compare<T: cmp::Ord>(x: &T, y: &T) -> i32 { return x.cmp(y); } fn main() {}",
        ),
        (
            "provider_num",
            "import <num>; fn main() { match num::checked_add_i32(1, 2) { std::Option::Some(x) -> {}, std::Option::None -> {}, } }",
        ),
    ] {
        let source_check =
            check_source_semantics_at(&format!("{name}_source"), source, &source_root);
        let artifact_check =
            check_source_semantics_at(&format!("{name}_artifact"), source, &artifact_only_root);
        assert!(
            source_check.is_err(),
            "source-only sysroot retained removed spelling {name}: {source_check:?}"
        );
        assert!(
            artifact_check.is_err(),
            "artifact-only sysroot retained removed spelling {name}: {artifact_check:?}"
        );
    }
}

#[test]
fn owned_container_surface_is_canonical_and_matches_fresh_artifacts() {
    let fixture = include_str!("../../../tests/luna/stdlib/core/owned_container_surface.ln");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");
    let artifact_root = temp_sysroot("owned_container_artifact");
    copy_tree(&canonical_external, &artifact_root.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("artifact sysroot"))
        .build_all(true)
        .expect("fresh owned-container artifacts must build from current providers");

    let source_root = temp_sysroot("owned_container_source_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("owned_container_artifact_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);

    let source_result =
        compile_and_run_source_at("owned_container_source_only", fixture, &source_root);
    let artifact_result =
        compile_and_run_source_at("owned_container_artifact_only", fixture, &artifact_only_root);
    assert_eq!(
        source_result,
        Ok(0),
        "canonical owned-container source surface should execute: {source_result:?}"
    );
    assert_eq!(
        artifact_result,
        Ok(0),
        "canonical owned-container artifact surface should execute: {artifact_result:?}"
    );
    assert_eq!(
        source_result, artifact_result,
        "owned-container source and artifact behavior must agree"
    );

    for (name, source) in [
        (
            "bare_vec",
            "import <vec>; fn main() { dec value: Vec<i32> = std::vec_new<i32>(); }",
        ),
        (
            "bare_string",
            "import <string>; fn main() { dec value: String = std::string_new(); }",
        ),
        (
            "bare_box",
            "import <box>; fn main() { dec value: Box<i32> = std::box_new<i32>(1); }",
        ),
        (
            "provider_vec",
            "import <vec>; fn main() { dec value: vec::Vec<i32> = std::vec_new<i32>(); }",
        ),
        (
            "provider_string",
            "import <string>; fn main() { dec value: string::String = std::string_new(); }",
        ),
        (
            "provider_box",
            "import <box>; fn main() { dec value: box::Box<i32> = std::box_new<i32>(1); }",
        ),
    ] {
        let source_check =
            check_source_semantics_at(&format!("{name}_source"), source, &source_root);
        let artifact_check =
            check_source_semantics_at(&format!("{name}_artifact"), source, &artifact_only_root);
        assert!(
            source_check.is_err(),
            "source-only sysroot retained removed owned-container path {name}: {source_check:?}"
        );
        assert!(
            artifact_check.is_err(),
            "artifact-only sysroot retained removed owned-container path {name}: {artifact_check:?}"
        );
    }
}

#[test]
fn collection_surface_is_canonical_and_matches_fresh_artifacts() {
    let fixture = include_str!("../../../tests/luna/stdlib/core/collection_surface.ln");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");
    let artifact_root = temp_sysroot("collection_surface_artifact");
    copy_tree(&canonical_external, &artifact_root.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("artifact sysroot"))
        .build_all(true)
        .expect("fresh collection artifacts must build from current providers");

    let source_root = temp_sysroot("collection_surface_source_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("collection_surface_artifact_only");
    copy_tree(
        &artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);

    let source_result = compile_and_run_source_at("collection_surface_source_only", fixture, &source_root);
    let artifact_result =
        compile_and_run_source_at("collection_surface_artifact_only", fixture, &artifact_only_root);
    assert_eq!(source_result, Ok(0), "collection source mode must execute: {source_result:?}");
    assert_eq!(
        artifact_result,
        Ok(0),
        "collection artifact-only mode must execute without source fallback: {artifact_result:?}"
    );
    assert_eq!(source_result, artifact_result, "collection source/artifact behavior must agree");

    for (name, fixture) in [
        (
            "legacy_hashmap",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashmap.ln"),
        ),
        (
            "legacy_hashmap_constructor",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashmap_constructor.ln"),
        ),
        (
            "legacy_hashset",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashset.ln"),
        ),
        (
            "legacy_hashset_constructor",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashset_constructor.ln"),
        ),
        (
            "legacy_iter_collect_path",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_iter_collect_path.ln"),
        ),
        (
            "provider_derived_collection_namespaces",
            include_str!("../../../tests/luna/stdlib/phase3/reject_provider_derived_collection_namespaces.ln"),
        ),
        (
            "public_raw_table_surface",
            include_str!("../../../tests/luna/stdlib/phase3/reject_public_raw_table_surface.ln"),
        ),
        (
            "unqualified_raw_table_surface",
            include_str!("../../../tests/luna/stdlib/phase3/reject_unqualified_raw_table_surface.ln"),
        ),
    ] {
        let source_check = check_source_semantics_at(
            &format!("{name}_source_only"),
            fixture,
            &source_root,
        );
        let artifact_check = check_source_semantics_at(
            &format!("{name}_artifact_only"),
            fixture,
            &artifact_only_root,
        );
        assert!(source_check.is_err(), "source sysroot retained rejected path {name}");
        assert!(artifact_check.is_err(), "artifact-only sysroot retained rejected path {name}");
    }
}

#[test]
fn utility_surface_and_cross_provider_iter_module_match_fresh_artifacts() {
    let fixture = include_str!("../../../tests/luna/stdlib/core/utility_surface.ln");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");

    let fresh_artifact_root = temp_sysroot("utility_surface_fresh_artifacts");
    copy_tree(&canonical_external, &fresh_artifact_root.join("libs/external"));
    SysrootBuilder::new(
        Sysroot::from_root(fresh_artifact_root.clone()).expect("fresh artifact sysroot"),
    )
    .build_all(true)
    .expect("utility providers must build fresh canonical artifacts");

    let source_root = temp_sysroot("utility_surface_source_only");
    copy_tree(
        &fresh_artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("utility_surface_artifact_only");
    copy_tree(
        &fresh_artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);

    let source_result =
        compile_and_run_output_at("utility_surface_source_only", fixture, &source_root);
    let artifact_result =
        compile_and_run_output_at("utility_surface_artifact_only", fixture, &artifact_only_root);
    assert_eq!(source_result, artifact_result, "source/artifact behavior must agree");
    let normalized_output = source_result.map(|(code, stdout, stderr)| {
        (code, stdout.replace("\r\n", "\n"), stderr)
    });
    assert_eq!(
        normalized_output,
        Ok((0, "utility module aggregatio\n".into(), String::new())),
        "fresh source/artifact utility outputs must match and execute without source fallback"
    );

    for (name, fixture) in [
        (
            "legacy_ptr_path",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_ptr_path.ln"),
        ),
        (
            "legacy_mem_path",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_mem_path.ln"),
        ),
        (
            "legacy_slice_module_path",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_slice_module_path.ln"),
        ),
        (
            "legacy_root_utility_names",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_utility_names.ln"),
        ),
        (
            "legacy_root_slice_helper",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_slice_helper.ln"),
        ),
        (
            "legacy_root_iter_consumer",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_iter_consumer.ln"),
        ),
        (
            "provider_derived_utility_namespaces",
            include_str!("../../../tests/luna/stdlib/core/reject_provider_derived_utility_namespaces.ln"),
        ),
        (
            "legacy_root_panic",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_panic.ln"),
        ),
        (
            "private_utf8_helper_path",
            include_str!("../../../tests/luna/stdlib/core/reject_private_utf8_helper_path.ln"),
        ),
    ] {
        let source_check =
            check_source_semantics_at(&format!("{name}_source_only"), fixture, &source_root);
        let artifact_check = check_source_semantics_at(
            &format!("{name}_artifact_only"),
            fixture,
            &artifact_only_root,
        );
        assert!(source_check.is_err(), "source-only sysroot retained {name}: {source_check:?}");
        assert!(
            artifact_check.is_err(),
            "artifact-only sysroot retained {name}: {artifact_check:?}"
        );
    }
}

#[test]
fn global_std_namespace_closure_matches_fresh_source_and_artifact_only_sysroots() {
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external");

    let fresh_artifact_root = temp_sysroot("global_namespace_fresh_artifacts");
    copy_tree(&canonical_external, &fresh_artifact_root.join("libs/external"));
    SysrootBuilder::new(
        Sysroot::from_root(fresh_artifact_root.clone()).expect("fresh global sysroot"),
    )
    .build_all(true)
    .expect("the complete canonical sysroot must rebuild from current sources");

    let source_root = temp_sysroot("global_namespace_source_only");
    copy_tree(
        &fresh_artifact_root.join("libs/external"),
        &source_root.join("libs/external"),
    );
    remove_artifacts_recursively(&source_root);

    let artifact_only_root = temp_sysroot("global_namespace_artifact_only");
    copy_tree(
        &fresh_artifact_root.join("libs/external"),
        &artifact_only_root.join("libs/external"),
    );
    remove_sources_recursively(&artifact_only_root);
    assert_eq!(
        count_extension_recursively(&artifact_only_root, "ln"),
        0,
        "artifact-only mode must make source fallback impossible"
    );

    let positives = [
        (
            "question_mark_contracts",
            include_str!("../../../tests/luna/language/try_codegen_canonical_std_try.ln"),
        ),
        (
            "ordinary_core",
            include_str!("../../../tests/luna/stdlib/core/ordinary_core_surface.ln"),
        ),
        (
            "try_convert_float_and_panic",
            include_str!("../../../tests/luna/stdlib/core/global_namespace_try_convert.ln"),
        ),
        (
            "broad_provider_load",
            include_str!("../../../tests/luna/stdlib/core/global_provider_load.ln"),
        ),
        (
            "language_contract_impl_paths",
            include_str!("../../../tests/luna/stdlib/core/language_contract_impl_paths.ln"),
        ),
        (
            "owned_containers",
            include_str!("../../../tests/luna/stdlib/core/owned_container_surface.ln"),
        ),
        (
            "collections",
            include_str!("../../../tests/luna/stdlib/core/collection_surface.ln"),
        ),
        (
            "utilities",
            include_str!("../../../tests/luna/stdlib/core/utility_surface.ln"),
        ),
        (
            "path",
            include_str!("../../../tests/luna/stdlib/path/path_semantics_v1.ln"),
        ),
        (
            "io",
            include_str!("../../../tests/luna/stdlib/io/io_runtime.ln"),
        ),
    ];

    for (name, source) in positives {
        let source_result = compile_and_run_output_at(
            &format!("global_{name}_source"),
            source,
            &source_root,
        );
        let artifact_result = compile_and_run_output_at(
            &format!("global_{name}_artifact"),
            source,
            &artifact_only_root,
        );
        assert_eq!(
            source_result,
            artifact_result,
            "canonical {name} behavior must match with source-only and artifact-only providers"
        );
        assert_eq!(
            source_result.as_ref().map(|(code, _, _)| *code),
            Ok(0),
            "canonical {name} fixture must compile, link, and execute: {source_result:?}"
        );
    }

    // Reverse the two providers that contribute to std::iter without changing
    // the consumer program; this checks that the merge is not import-order dependent.
    let utility = include_str!("../../../tests/luna/stdlib/core/utility_surface.ln");
    let reversed_utility = utility
        .replace("import <iter_adapters>;", "import <__iter_adapters_temp>;")
        .replace("import <iter_consumers>;", "import <iter_adapters>;")
        .replace("import <__iter_adapters_temp>;", "import <iter_consumers>;");
    for (mode, root) in [("source", &source_root), ("artifact", &artifact_only_root)] {
        let result = compile_and_run_output_at(
            &format!("global_iter_reversed_{mode}"),
            &reversed_utility,
            root,
        );
        assert_eq!(
            result.as_ref().map(|(code, _, _)| *code),
            Ok(0),
            "std::iter contributions must resolve after reversed provider import order: {result:?}"
        );
    }

    let negatives = [
        (
            "root_option",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_option.ln"),
        ),
        (
            "root_iterator",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_iterator.ln"),
        ),
        (
            "root_try",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_try.ln"),
        ),
        (
            "root_drop",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_drop.ln"),
        ),
        (
            "root_copy",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_copy.ln"),
        ),
        (
            "root_path_namespace",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_path_namespace.ln"),
        ),
        (
            "root_io_namespace",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_io_namespace.ln"),
        ),
        (
            "root_ptr_path",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_ptr_path.ln"),
        ),
        (
            "root_mem_path",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_mem_path.ln"),
        ),
        (
            "root_slice_path",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_slice_module_path.ln"),
        ),
        (
            "root_slice_function",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_slice_helper.ln"),
        ),
        (
            "root_iterator_function",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_iter_consumer.ln"),
        ),
        (
            "provider_iterator_namespaces",
            include_str!("../../../tests/luna/stdlib/core/reject_provider_derived_utility_namespaces.ln"),
        ),
        (
            "root_panic",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_panic.ln"),
        ),
        (
            "private_utf8_module",
            include_str!("../../../tests/luna/stdlib/core/reject_private_utf8_helper_path.ln"),
        ),
        (
            "root_hashmap",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashmap.ln"),
        ),
        (
            "root_hashmap_constructor",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashmap_constructor.ln"),
        ),
        (
            "root_hashset",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashset.ln"),
        ),
        (
            "root_hashset_constructor",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_hashset_constructor.ln"),
        ),
        (
            "provider_collection_namespaces",
            include_str!("../../../tests/luna/stdlib/phase3/reject_provider_derived_collection_namespaces.ln"),
        ),
        (
            "legacy_iter_collect_path",
            include_str!("../../../tests/luna/stdlib/phase3/reject_legacy_iter_collect_path.ln"),
        ),
        (
            "public_raw_table",
            include_str!("../../../tests/luna/stdlib/phase3/reject_public_raw_table_surface.ln"),
        ),
    ];

    let mut accepted_legacy_spellings = Vec::new();
    for (name, source) in negatives {
        for (mode, root) in [("source", &source_root), ("artifact", &artifact_only_root)] {
            let result = check_source_semantics_at(
                &format!("global_{name}_{mode}"),
                source,
                root,
            );
            if matches!(name, "root_drop" | "root_copy") {
                assert!(
                    result
                        .as_ref()
                        .err()
                        .is_some_and(|diagnostics| diagnostics.contains("cannot resolve trait path")),
                    "bare `{name}` must fail specifically during trait-path resolution in {mode} mode: {result:?}"
                );
            }
            if result.is_ok() {
                accepted_legacy_spellings.push(format!("{name} ({mode})"));
            }
        }
    }
    assert!(
        accepted_legacy_spellings.is_empty(),
        "legacy public spellings must be rejected in both modes; unexpectedly accepted: {accepted_legacy_spellings:?}"
    );
}

fn count_extension_recursively(root: &Path, extension: &str) -> usize {
    fs::read_dir(root)
        .expect("enumerate isolated sysroot")
        .map(|entry| entry.expect("read isolated sysroot entry").path())
        .map(|path| {
            if path.is_dir() {
                count_extension_recursively(&path, extension)
            } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
                1
            } else {
                0
            }
        })
        .sum()
}
