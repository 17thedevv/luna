#[path = "support/stdlib.rs"]
mod stdlib;
use std::path::Path;
use std::process::Command;
use stdlib::{render, workspace_root, ProviderModes};

fn assert_cli_output(modes: &ProviderModes, relative: &str, expected_stdout: &str) {
    let fixture = workspace_root()
        .join("tests/luna/stdlib/core")
        .join(relative);
    let mut results = Vec::new();
    for (name, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let tag = format!(
            "{name}_{}",
            Path::new(relative).file_stem().unwrap().to_str().unwrap()
        );
        let (executable, build) = modes.build(root, &fixture, &tag);
        assert!(
            build.status.success(),
            "{name}/{relative}: {}",
            render(&build)
        );
        let output = Command::new(executable).output().unwrap();
        assert!(
            output.status.success(),
            "{name}/{relative}: {}",
            render(&output)
        );
        // A lossy UTF-8 conversion would hide precisely the invariant under test.
        let text = std::str::from_utf8(&output.stdout)
            .unwrap_or_else(|error| panic!("{name}/{relative} emitted invalid UTF-8: {error}"));
        assert_eq!(
            text.replace("\r\n", "\n"),
            expected_stdout,
            "{name}/{relative}"
        );
        assert!(
            output.stderr.is_empty(),
            "{name}/{relative}: {}",
            render(&output)
        );
        results.push((output.status.code(), output.stdout, output.stderr));
    }
    assert_eq!(
        results[0], results[1],
        "source/artifact observable parity: {relative}"
    );
}

#[test]
fn formatting_v1_matches_source_and_fresh_artifacts_through_cli() {
    let modes = ProviderModes::fresh();
    assert_cli_output(&modes,
        "formatting_v1.ln",
        "true|false|0|12345|123456789|18446744073709551615|123|12345|-128|-12345|-123456789|-123456789|-123|-12345|界|Việt|-12,34\n",
    );
    assert_cli_output(&modes,
        "formatting_integer_boundaries_v1.ln",
        "127|-128|32767|-32768|2147483647|-2147483648|9223372036854775807|-9223372036854775808|18446744073709551615|340282366920938463463374607431768211455|-170141183460469231731687303715884105728|170141183460469231731687303715884105727|18446744073709551615|-9223372036854775808|\n",
    );
    assert_cli_output(&modes,
        "formatting_integer_all_edges_v1.ln",
        "0|255|0|65535|0|4294967295|0|18446744073709551615|0|340282366920938463463374607431768211455|0|18446744073709551615|-128|127|-32768|32767|-2147483648|2147483647|-9223372036854775808|9223372036854775807|-170141183460469231731687303715884105728|170141183460469231731687303715884105727|-9223372036854775808|9223372036854775807|9|10|11|99|100|101|999|1000|1001|0|1|255|0|1|65535|0|1|4294967295|0|1|18446744073709551615|0|1|340282366920938463463374607431768211455|0|1|18446744073709551615|-128|-1|0|1|127|-32768|-1|0|1|32767|-2147483648|-1|0|1|2147483647|-9223372036854775808|-1|0|1|9223372036854775807|-170141183460469231731687303715884105728|-1|0|1|170141183460469231731687303715884105727|-9223372036854775808|-1|0|1|9223372036854775807|\n",
    );
    assert_cli_output(&modes, "formatting_unicode_v1.ln", "prefix:é߿ࠀ𐀀􏿿\0多字\n");
    assert_cli_output(&modes, "formatting_custom_writer_v1.ln", "");
    assert_cli_output(&modes, "formatting_writer_failure_v1.ln", "");
    assert_cli_output(&modes, "formatting_writer_failure_matrix_v1.ln", "");
    assert_cli_output(
        &modes,
        "formatting_file_error_v1.ln",
        "file not found|permission denied|invalid input|I/O error\n",
    );
}

#[test]
fn formatting_v1_negative_contracts_reject_through_cli_in_both_modes() {
    let modes = ProviderModes::fresh();
    for (fixture, expected) in [
        ("reject_display_byte_array.ln", "Display"),
        ("reject_display_f32.ln", "Display"),
        ("reject_display_f64.ln", "Display"),
        ("reject_legacy_root_display.ln", "fmt"),
    ] {
        let fixture_path = workspace_root()
            .join("tests/luna/stdlib/core")
            .join(fixture);
        let mut diagnostics = Vec::new();
        for (name, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            for command in ["check", "build"] {
                let output = if command == "check" {
                    modes.check(root, &fixture_path)
                } else {
                    let (executable, output) =
                        modes.build(root, &fixture_path, &format!("{name}_{fixture}"));
                    assert!(
                        !executable.exists(),
                        "invalid program must not produce an executable"
                    );
                    output
                };
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{name}/{fixture}/{command}: {}",
                    render(&output)
                );
                let message = std::str::from_utf8(&output.stderr).unwrap();
                assert!(
                    message.contains(expected),
                    "{name}/{fixture}/{command}: {message}"
                );
                assert!(!message.contains("panicked"), "{message}");
                assert!(output.stdout.is_empty());
                diagnostics.push(stdlib::diagnostic_messages(&output));
            }
        }
        assert_eq!(
            diagnostics[0], diagnostics[1],
            "check/build diagnostics: {fixture}"
        );
        assert_eq!(
            diagnostics[0], diagnostics[2],
            "source/artifact diagnostics: {fixture}"
        );
        assert_eq!(
            diagnostics[0], diagnostics[3],
            "source/artifact build diagnostics: {fixture}"
        );
    }
}
