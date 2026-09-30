use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("luna-cli must live below the workspace root")
        .to_path_buf()
}

fn run_fixture(relative: &str) -> Output {
    let root = workspace_root();
    let source = root
        .join("tests")
        .join("luna")
        .join("stdlib")
        .join("core")
        .join(relative);
    let executable = std::env::temp_dir().join(format!(
        "luna_fmt_cli_{}_{}.exe",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let build = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .arg("--quiet")
        .arg("-I")
        .arg(&root)
        .output()
        .unwrap_or_else(|error| panic!("failed to build {relative} with luna CLI: {error}"));
    assert!(
        build.status.success(),
        "luna build failed for {relative}:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(&executable)
        .output()
        .unwrap_or_else(|error| panic!("failed to execute {relative}: {error}"));
    let _ = std::fs::remove_file(executable);
    output
}

fn assert_cli_output(relative: &str, expected_stdout: &str) {
    let output = run_fixture(relative);
    assert!(
        output.status.success(),
        "luna run failed for {relative}:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        expected_stdout,
        "unexpected CLI output for {relative}"
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected CLI stderr for {relative}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn formatting_v1_executes_through_the_public_luna_cli() {
    assert_cli_output(
        "formatting_v1.ln",
        "true|false|0|12345|123456789|18446744073709551615|123|12345|-128|-12345|-123456789|-123456789|-123|-12345|界|Việt|-12,34\n",
    );
    assert_cli_output(
        "formatting_integer_boundaries_v1.ln",
        "127|-128|32767|-32768|2147483647|-2147483648|9223372036854775807|-9223372036854775808|18446744073709551615|340282366920938463463374607431768211455|-170141183460469231731687303715884105728|170141183460469231731687303715884105727|18446744073709551615|-9223372036854775808|\n",
    );
    assert_cli_output(
        "formatting_integer_all_edges_v1.ln",
        "0|255|0|65535|0|4294967295|0|18446744073709551615|0|340282366920938463463374607431768211455|0|18446744073709551615|-128|127|-32768|32767|-2147483648|2147483647|-9223372036854775808|9223372036854775807|-170141183460469231731687303715884105728|170141183460469231731687303715884105727|-9223372036854775808|9223372036854775807|9|10|11|99|100|101|999|1000|1001|0|1|255|0|1|65535|0|1|4294967295|0|1|18446744073709551615|0|1|340282366920938463463374607431768211455|0|1|18446744073709551615|-128|-1|0|1|127|-32768|-1|0|1|32767|-2147483648|-1|0|1|2147483647|-9223372036854775808|-1|0|1|9223372036854775807|-170141183460469231731687303715884105728|-1|0|1|170141183460469231731687303715884105727|-9223372036854775808|-1|0|1|9223372036854775807|\n",
    );
    assert_cli_output("formatting_unicode_v1.ln", "prefix:é߿ࠀ𐀀􏿿\0多字\n");
    assert_cli_output("formatting_custom_writer_v1.ln", "");
    assert_cli_output("formatting_writer_failure_v1.ln", "");
    assert_cli_output(
        "formatting_file_error_v1.ln",
        "file not found|permission denied|invalid input|I/O error\n",
    );
}
