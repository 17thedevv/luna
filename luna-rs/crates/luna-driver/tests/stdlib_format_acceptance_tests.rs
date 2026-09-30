use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{CompilerOptions, compile};
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

fn fresh_source_and_artifact_roots(tag: &str) -> (PathBuf, PathBuf) {
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

fn rejected_messages(root: &Path, fixture: &str, tag: &str) -> Vec<String> {
    let work = temp_root(tag);
    fs::create_dir_all(&work).unwrap();
    let source = work.join("negative.ln");
    let executable = work.join("negative.exe");
    fs::write(&source, fixture).unwrap();
    let options = CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        search_paths: vec![root.to_string_lossy().into_owned()],
        quiet: true,
        ..Default::default()
    };
    compile(&source.to_string_lossy(), fixture.to_string(), &options)
        .expect_err("outside-scope Display program must fail full compilation")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
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
            "-128|-1|9|10|11|0|127|0|9|10|11|255|-32768|-1|9|10|11|0|32767|0|9|10|11|999|1000|1001|65535|-2147483648|-1|9|10|11|0|2147483647|0|9|10|11|999999999|1000000000|1000000001|4294967295|-9223372036854775808|-1|9|10|11|0|9223372036854775807|0|9|10|11|999999999999|1000000000000|1000000000001|18446744073709551615|-170141183460469231731687303715884105728|-1|9|10|11|0|170141183460469231731687303715884105727|0|9|10|11|999|1000|1001|340282366920938463463374607431768211455|-9223372036854775808|-1|9|10|11|0|9223372036854775807|0|9|10|11|18446744073709551615|\n",
        ),
        (
            "formatting_unicode_v1.ln",
            include_str!("../../../tests/luna/stdlib/core/formatting_unicode_v1.ln"),
            "\u{7f}|\u{80}|\u{7ff}|\u{800}|\u{ffff}|\u{10000}|\u{10ffff}|\0|A\0B||\n",
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
    let (source_only, artifact_only) = fresh_source_and_artifact_roots("positive");

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
fn formatting_v1_negative_cases_reject_consistently_from_source_and_artifact() {
    let cases = [
        (
            "byte_array",
            include_str!("../../../tests/luna/stdlib/core/reject_display_byte_array.ln"),
            "display",
        ),
        (
            "f32",
            include_str!("../../../tests/luna/stdlib/core/reject_display_f32.ln"),
            "display",
        ),
        (
            "f64",
            include_str!("../../../tests/luna/stdlib/core/reject_display_f64.ln"),
            "display",
        ),
        (
            "legacy_root_trait",
            include_str!("../../../tests/luna/stdlib/core/reject_legacy_root_display.ln"),
            "display",
        ),
    ];
    let (source_only, artifact_only) = fresh_source_and_artifact_roots("negative");

    for (name, fixture, expected_diagnostic) in cases {
        let source_messages = rejected_messages(&source_only, fixture, &format!("source_{name}"));
        let artifact_messages =
            rejected_messages(&artifact_only, fixture, &format!("artifact_{name}"));
        assert!(
            !source_messages.is_empty(),
            "{name} unexpectedly compiled from source"
        );
        assert_eq!(
            source_messages, artifact_messages,
            "source and artifact diagnostics diverged for {name}"
        );
        assert!(
            source_messages.iter().any(|message| {
                message
                    .to_lowercase()
                    .contains(&expected_diagnostic.to_lowercase())
            }),
            "{name} must reject for {expected_diagnostic}, got: {source_messages:#?}"
        );
    }
}
