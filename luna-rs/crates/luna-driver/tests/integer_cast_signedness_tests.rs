use std::fs;
use std::process::Command;

fn assert_fixture_runs(source_name: &str, fixture: &str) {
    let work = std::env::temp_dir().join(format!(
        "luna_integer_cast_{source_name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&work).unwrap();
    let source = work.join("integer_cast_signedness.ln");
    let executable = work.join(if cfg!(windows) {
        "integer_cast_signedness.exe"
    } else {
        "integer_cast_signedness"
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
fn explicit_integer_widening_preserves_source_signedness() {
    assert_fixture_runs(
        "unsigned_and_signed_widening",
        include_str!("../../../tests/luna/compiler/integer_cast_signedness.ln"),
    );
}
