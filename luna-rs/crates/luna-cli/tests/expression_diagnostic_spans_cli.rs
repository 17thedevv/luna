#[path = "support/stdlib.rs"]
mod stdlib;
use stdlib::{render, workspace_root, ProviderModes};

#[test]
fn invalid_expression_diagnostics_use_the_actual_utf8_source_line() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/diagnostic_spans");
    for (name, code, line, token) in [
        ("deref_scalar", "E2002", 4, "value"),
        ("neg_bool", "E2012", 4, "value"),
        ("tuple_scalar", "E2003", 4, "value"),
        ("index_scalar", "E2003", 4, "value"),
        ("char_invalid", "E2026", 3, "55296"),
    ] {
        let fixture = fixtures.join(format!("{name}.ln"));
        let source = std::fs::read_to_string(&fixture).unwrap();
        let offending_line = source.lines().nth(line - 1).unwrap();
        let column = offending_line.find(token).unwrap() + 1;
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            for command in ["check", "build"] {
                let output = if command == "check" {
                    modes.check(root, &fixture)
                } else {
                    let (executable, output) = modes.build(root, &fixture, &format!("{mode}_{name}"));
                    assert!(!executable.exists());
                    output
                };
                assert_eq!(output.status.code(), Some(1), "{}", render(&output));
                let stderr = String::from_utf8(output.stderr).unwrap();
                assert!(stderr.contains(&format!("error[{code}]")), "{stderr}");
                assert!(stderr.contains(&format!(":{line}:{column}")), "{mode}/{command}/{name}: {stderr}");
                assert!(!stderr.contains("E_UNRESOLVED_INFERENCE"), "{stderr}");
            }
        }
    }
}
