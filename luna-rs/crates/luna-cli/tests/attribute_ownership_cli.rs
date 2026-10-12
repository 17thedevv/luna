//! E1-DIAG-OWNERSHIP-01: real source, macro and provider admission boundaries.
#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{diagnostic_messages, render, workspace_root, ProviderModes};

#[test]
fn attribute_admission_preserves_codes_spans_and_check_build_parity() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/diagnostic_ownership");
    for (name, code) in [
        ("unknown_attribute", "E0005"), ("malformed_attribute", "E0005"),
        ("reserved_intrinsic_declaration", "E1009"), ("reserved_intrinsic_link", "E1009"),
        ("macro_reserved_declaration", "E1009"), ("macro_link_main", "E2032"),
        ("invalid_application", "E2032"),
        ("macro_as_value", "E2001"), ("invalid_integer_syntax", "E0003"),
        ("macro_rule_mismatch", "E1010"), ("macro_recursion", "E1010"),
        ("unsupported_comptime_statement", "E4005"),
    ] {
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let input = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &input);
            let (exe, build) = modes.build(root, &input, &format!("owner_{name}_{mode}"));
            assert!(!exe.exists(), "negative published executable: {name}/{mode}");
            for output in [&check, &build] {
                assert_eq!(output.status.code(), Some(1), "{name}/{mode}: {}", render(output));
                let messages = diagnostic_messages(output);
                assert_eq!(messages.len(), 1, "{name}/{mode}: {}", render(output));
                assert!(messages[0].contains(&format!("error[{code}]")), "{}", render(output));
                assert!(String::from_utf8_lossy(&output.stderr).contains("-->"), "{}", render(output));
            }
            assert_eq!(diagnostic_messages(&check), diagnostic_messages(&build));
        }
    }

    // User providers are independently admitted in their own Resolver context.
    // Invalid providers must fail compilation/publication; they cannot be
    // manufactured as successful artifacts to claim artifact negative parity.
    let project = modes.source.join("ownership_project");
    fs::create_dir(&project).unwrap();
    // Existing recursion allowance remains usable; the CLI must reach the
    // admission limit rather than exhausting its platform main-thread stack.
    let mut near_limit = String::from("macro m0 { () => { 0 } }\n");
    for depth in 1..=100 {
        near_limit.push_str(&format!("macro m{depth} {{ () => {{ m{}!() }} }}\n", depth - 1));
    }
    near_limit.push_str("fn main()->i32 { return m100!(); }");
    let valid = project.join("near_limit.ln");
    fs::write(&valid, near_limit).unwrap();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let check = modes.check(root, &valid);
        let (exe, build) = modes.build(root, &valid, &format!("owner_near_limit_{mode}"));
        assert!(check.status.success(), "{}", render(&check));
        assert!(build.status.success(), "{}", render(&build));
        assert_eq!(Command::new(&exe).output().unwrap().status.code(), Some(0));
        let ordinary = fixtures.join("positive_nested_main_link.ln");
        let check = modes.check(root, &ordinary);
        let (exe, build) = modes.build(root, &ordinary, &format!("owner_nested_main_{mode}"));
        assert!(check.status.success(), "{}", render(&check));
        assert!(build.status.success(), "{}", render(&build));
        assert_eq!(Command::new(&exe).output().unwrap().status.code(), Some(0));
    }
    for (tag, provider, code) in [
        ("declaration", "export fn __luna_intrinsic_WrappingAdd() {}", "E1009"),
        ("link", "#[link(\"__luna_intrinsic_WrappingAdd\")] extern fn forged();", "E1009"),
        ("module", "export module nested { export fn __luna_intrinsic_WrappingAdd() {} }", "E1009"),
        ("macro_reserved", "macro reserve { () => { export fn __luna_intrinsic_WrappingAdd() {} } } reserve!();", "E1009"),
        ("macro_application", "macro invalid { () => { #[test] export struct Invalid {}; } } invalid!();", "E2032"),
        ("macro_rule", "macro only { () => { 0 } } export fn f()->i32 { return only!(1); }", "E1010"),
        ("comptime_statement", "export fn f() { comptime { 1i32 + 1i32; } }", "E4005"),
    ] {
        let source = project.join("provider.ln");
        let consumer = project.join("consumer.ln");
        fs::write(&source, provider).unwrap();
        fs::write(&consumer, "import \"provider\"; fn main() -> i32 { return 0; }").unwrap();
        let check = modes.check(&modes.source, &consumer);
        let (exe, build) = modes.build(&modes.source, &consumer, &format!("owner_provider_{tag}"));
        assert!(!exe.exists());
        for output in [&check, &build] {
            assert_eq!(output.status.code(), Some(1), "{}", render(output));
            let messages = diagnostic_messages(output);
            assert_eq!(messages.len(), 1, "{}", render(output));
            assert!(messages[0].contains(&format!("error[{code}]")), "{}", render(output));
            assert!(String::from_utf8_lossy(&output.stderr).contains("-->"), "{}", render(output));
            let stderr = String::from_utf8_lossy(&output.stderr);
            let primary = stderr.lines().find(|line| line.trim_start().starts_with("-->")).unwrap();
            assert!(primary.contains("provider.ln:"), "original provider primary span: {}", render(output));
        }
        assert_eq!(diagnostic_messages(&check), diagnostic_messages(&build));
        let output = Command::new(env!("CARGO_BIN_EXE_luna"))
            .arg("build").arg(&source).args(["--lib", "--emit", "llib", "--quiet", "-o"])
            .arg(project.join(format!("invalid_{tag}.llib")))
            .env("LUNA_SYSROOT", &modes.source).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{}", render(&output));
        assert!(!project.join(format!("invalid_{tag}.llib")).exists());
    }
}
