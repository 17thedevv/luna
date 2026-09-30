use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{CompilerOptions, check_semantic_only};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn canonical_external() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs/external")
}

fn temp_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "luna_fmt_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn remove_extension_recursively(root: &Path, extension: &str) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            remove_extension_recursively(&path, extension);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            fs::remove_file(path).unwrap();
        }
    }
}

fn fresh_provider_modes(tag: &str) -> (PathBuf, PathBuf) {
    let artifacts = temp_root(&format!("{tag}_fresh_artifacts"));
    copy_tree(&canonical_external(), &artifacts.join("libs/external"));
    SysrootBuilder::new(Sysroot::from_root(artifacts.clone()).unwrap())
        .build_all(true)
        .expect("fresh .llib/.obj providers must build from current source");

    let source_only = temp_root(&format!("{tag}_source_only"));
    copy_tree(
        &artifacts.join("libs/external"),
        &source_only.join("libs/external"),
    );
    remove_extension_recursively(&source_only, "llib");
    remove_extension_recursively(&source_only, "obj");

    let artifact_only = temp_root(&format!("{tag}_artifact_only"));
    copy_tree(
        &artifacts.join("libs/external"),
        &artifact_only.join("libs/external"),
    );
    remove_extension_recursively(&artifact_only, "ln");

    (source_only, artifact_only)
}

fn run_fixture(root: &Path, fixture: &str, tag: &str) -> (i32, String, String) {
    let work = temp_root(tag);
    fs::create_dir_all(&work).unwrap();
    let source = work.join("formatting.ln");
    let executable = work.join("formatting.exe");
    fs::write(&source, fixture).unwrap();
    let options = CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        no_link: false,
        ..Default::default()
    };
    luna_driver::compile(&source.to_string_lossy(), fixture.to_string(), &options)
        .unwrap_or_else(|diagnostics| panic!("format fixture must compile: {diagnostics:#?}"));
    let output = Command::new(executable).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn compile_diagnostics(root: &Path, fixture: &str, tag: &str) -> Vec<String> {
    let work = temp_root(tag);
    fs::create_dir_all(&work).unwrap();
    let source = work.join("negative.ln");
    fs::write(&source, fixture).unwrap();
    let options = CompilerOptions {
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        ..Default::default()
    };
    match check_semantic_only(&source.to_string_lossy(), fixture.to_string(), &options) {
        Err(diagnostics) => diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect(),
        Ok(()) => panic!("invalid formatting fixture passed semantic analysis: {tag}"),
    }
}

fn normalize_session_local_type_ids(message: &str) -> String {
    let mut normalized = String::with_capacity(message.len());
    let mut remaining = message;
    while let Some(start) = remaining.find("SemanticTypeId(") {
        normalized.push_str(&remaining[..start]);
        let id_start = start + "SemanticTypeId(".len();
        let Some(id_end_relative) = remaining[id_start..].find(')') else {
            normalized.push_str(&remaining[start..]);
            return normalized;
        };
        let id_end = id_start + id_end_relative;
        if !remaining[id_start..id_end].bytes().all(|byte| byte.is_ascii_digit()) {
            normalized.push_str(&remaining[start..=id_end]);
        } else {
            normalized.push_str("SemanticTypeId(<session-local>)");
        }
        remaining = &remaining[id_end + 1..];
    }
    normalized.push_str(remaining);
    normalized
}

#[test]
fn formatting_v1_matches_source_and_fresh_artifact_sysroots() {
    let fixtures = [
        (
            "formatting_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_v1.ln"),
            "true|false|0|12345|123456789|18446744073709551615|123|12345|-128|-12345|-123456789|-123456789|-123|-12345|界|Việt|-12,34\n",
        ),
        (
            "formatting_integer_boundaries_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_integer_boundaries_v1.ln"),
            "127|-128|32767|-32768|2147483647|-2147483648|9223372036854775807|-9223372036854775808|18446744073709551615|340282366920938463463374607431768211455|-170141183460469231731687303715884105728|170141183460469231731687303715884105727|18446744073709551615|-9223372036854775808|\n",
        ),
        (
            "formatting_integer_all_edges_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_integer_all_edges_v1.ln"),
            "0|255|0|65535|0|4294967295|0|18446744073709551615|0|340282366920938463463374607431768211455|0|18446744073709551615|-128|127|-32768|32767|-2147483648|2147483647|-9223372036854775808|9223372036854775807|-170141183460469231731687303715884105728|170141183460469231731687303715884105727|-9223372036854775808|9223372036854775807|9|10|11|99|100|101|999|1000|1001|0|1|255|0|1|65535|0|1|4294967295|0|1|18446744073709551615|0|1|340282366920938463463374607431768211455|0|1|18446744073709551615|-128|-1|0|1|127|-32768|-1|0|1|32767|-2147483648|-1|0|1|2147483647|-9223372036854775808|-1|0|1|9223372036854775807|-170141183460469231731687303715884105728|-1|0|1|170141183460469231731687303715884105727|-9223372036854775808|-1|0|1|9223372036854775807|\n",
        ),
        (
            "formatting_unicode_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_unicode_v1.ln"),
            "prefix:é߿ࠀ𐀀􏿿\0多字\n",
        ),
        (
            "formatting_custom_writer_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_custom_writer_v1.ln"),
            "",
        ),
        (
            "formatting_writer_failure_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_writer_failure_v1.ln"),
            "",
        ),
        (
            "formatting_file_error_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_file_error_v1.ln"),
            "file not found|permission denied|invalid input|I/O error\n",
        ),
    ];
    let (source_only, artifact_only) = fresh_provider_modes("positive");

    for (name, fixture, expected_stdout) in fixtures {
        let source_result = run_fixture(&source_only, fixture, &format!("source_{name}"));
        let artifact_result = run_fixture(&artifact_only, fixture, &format!("artifact_{name}"));
        assert_eq!(
            source_result, artifact_result,
            "source and fresh .llib behavior must agree for {name}"
        );
        assert_eq!(source_result.0, 0, "{name} failed: {source_result:?}");
        assert_eq!(
            source_result.1.replace("\r\n", "\n"),
            expected_stdout,
            "unexpected stdout for {name}"
        );
        assert!(source_result.2.is_empty(), "unexpected stderr for {name}");
    }
}

#[test]
fn formatting_v1_negative_cases_reject_for_the_expected_reason() {
    let (source_only, artifact_only) = fresh_provider_modes("negative");

    let cases = [
        (
            "byte_array",
            include_str!("../../../tests/luna/stdlib/core/reject_display_byte_array.ln"),
            "Display",
        ),
        (
            "f32",
            include_str!("../../../tests/luna/stdlib/core/reject_display_f32.ln"),
            "Display",
        ),
        (
            "f64",
            include_str!("../../../tests/luna/stdlib/core/reject_display_f64.ln"),
            "Display",
        ),
        (
            "legacy_root_trait",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_display.ln"),
            "fmt",
        ),
    ];

    for (name, fixture, expected_diagnostic) in cases {
        let mut source_diagnostics =
            compile_diagnostics(&source_only, fixture, &format!("source_{name}"));
        let mut artifact_diagnostics =
            compile_diagnostics(&artifact_only, fixture, &format!("artifact_{name}"));
        source_diagnostics = source_diagnostics
            .iter()
            .map(|message| normalize_session_local_type_ids(message))
            .collect();
        artifact_diagnostics = artifact_diagnostics
            .iter()
            .map(|message| normalize_session_local_type_ids(message))
            .collect();
        source_diagnostics.sort();
        artifact_diagnostics.sort();
        assert!(
            source_diagnostics.iter().any(|message| message
                .to_lowercase()
                .contains(&expected_diagnostic.to_lowercase())),
            "{name} source must fail for {expected_diagnostic}, got: {source_diagnostics:#?}"
        );
        assert_eq!(
            source_diagnostics, artifact_diagnostics,
            "source and artifact diagnostics must agree for {name}"
        );
    }
}
