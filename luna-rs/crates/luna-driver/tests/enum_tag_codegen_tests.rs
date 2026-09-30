use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "luna_enum_tag_codegen_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn assert_fixture_runs(source_name: &str, fixture: &str) {
    let work = temp_root(source_name);
    fs::create_dir_all(&work).unwrap();
    let source = work.join("enum_tag_match_codegen.ln");
    let executable = work.join(if cfg!(windows) {
        "enum_tag.exe"
    } else {
        "enum_tag"
    });
    fs::write(&source, fixture).unwrap();

    let options = luna_driver::CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        quiet: true,
        ..Default::default()
    };
    luna_driver::compile(&source.to_string_lossy(), fixture.to_string(), &options)
        .unwrap_or_else(|diagnostics| panic!("{source_name} must compile: {diagnostics:#?}"));

    let output = Command::new(executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{source_name}: {output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn enum_match_uses_resolved_tags_for_nested_and_duplicate_variant_names() {
    assert_fixture_runs(
        "enum_tag_match_codegen",
        include_str!("../../../tests/luna/compiler/enum_tag_match_codegen.ln"),
    );
    assert_fixture_runs(
        "nested_enum_variant_discrimination",
        include_str!("../../../tests/luna/compiler/nested_enum_variant_discrimination.ln"),
    );
}
