//! DIAG-7 behavioural coverage: every registered code whose condition is
//! reachable in this release reports its typed identity and its span policy.
//!
//! Codes whose condition cannot be reproduced in a test scope are declared with
//! a reason in `docs/tools/diag_producer_inventory.py`
//! (`UNREACHABLE_IN_TEST`), so the coverage map never claims more than the
//! mechanism provides.

#[path = "support/stdlib.rs"]
mod support;

use std::{fs, path::Path, process::Command};
use support::{render, ProviderModes};

fn fixture_dir() -> std::path::PathBuf {
    let root = luna_test_support::owned_temp_root().join("diag_per_code");
    fs::create_dir_all(&root).unwrap();
    root
}

/// Source diagnostics: the typed code must appear with a primary span.
#[test]
fn reachable_source_codes_report_typed_identity_with_a_primary_span() {
    let modes = ProviderModes::fresh();
    let fixtures = fixture_dir();
    let cases: [(&str, &str, &str); 10] = [
        (
            "E2007",
            "impl path that does not name a trait",
            "struct S { v: i32, };\nimpl S for i32 {\n    fn f(self) -> i32 { return 0; }\n}\nfn main() -> i32 { return 0; }\n",
        ),
        (
            "E2009",
            "infinitely sized recursive type",
            "struct A { a: A, };\nfn main() -> i32 { return 0; }\n",
        ),
        (
            "E2011",
            "assignment to a value that is not a place",
            "fn main() -> i32 {\n    1 = 2;\n    return 0;\n}\n",
        ),
        (
            "E2013",
            "binary operator rejected by the backend",
            "fn main() -> i32 {\n    dec r = 1..2;\n    return 0;\n}\n",
        ),
        (
            "E2017",
            "struct initializer missing a declared field",
            "struct S { x: i32, y: i32, };\nfn main() -> i32 {\n    dec s = S { x: 1 };\n    return s.x;\n}\n",
        ),
        (
            "E2018",
            "generic bound naming an unknown associated type",
            "trait HasItem { type Item; }\nfn f<X: HasItem<Missing = i32>>() -> i32 { return 0; }\nfn main() -> i32 { return 0; }\n",
        ),
        (
            "E2022",
            "match without the required wildcard arm",
            "fn main() -> i32 {\n    dec x = 5;\n    match x { 1 -> { return 0; } }\n}\n",
        ),
        (
            "E2027",
            "await outside an async function",
            "fn f() -> i32 { return 0; }\nfn main() -> i32 {\n    dec x = await f();\n    return x;\n}\n",
        ),
        (
            "E2028",
            "try operator without a residual context",
            "fn main() -> i32 {\n    dec x = 1?;\n    return x;\n}\n",
        ),
        (
            "E4001",
            "borrow of a local held across an await point",
            "async fn f() -> i32 { return 0; }\nasync fn g() -> i32 {\n    dec x = 1;\n    dec b = &x;\n    await f();\n    return *b;\n}\nfn main() -> i32 { return 0; }\n",
        ),
    ];

    let mut failures = Vec::new();
    for (code, condition, source) in cases {
        let path = fixtures.join(format!("{code}.ln"));
        fs::write(&path, source).unwrap();
        let output = modes.check(&modes.source, &path);
        let text = render(&output);
        let typed = text.contains(&format!("error[{code}]"));
        let primary_span = text.contains("-->") && text.contains('^');
        if !output.status.success() && typed && primary_span {
            continue;
        }
        failures.push(format!(
            "{code} ({condition}): typed={typed} primary_span={primary_span}\n{text}"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// Infrastructure diagnostics are not anchored to a source construct, so their
/// span policy is "no primary span" — but they must still carry a typed code
/// instead of a raw failure string.
#[test]
fn reachable_infrastructure_codes_report_typed_identity_without_a_source_span() {
    let cli = env!("CARGO_BIN_EXE_luna");
    let work = fixture_dir();
    let mut failures = Vec::new();

    // E6007: an emit kind the compiler does not implement.
    let program = work.join("ok.ln");
    fs::write(&program, "fn main() -> i32 { return 0; }\n").unwrap();
    let output = Command::new(cli)
        .arg("build")
        .arg(&program)
        .args(["--emit", "not-an-emit-kind", "--quiet", "-o"])
        .arg(work.join("emit.exe"))
        .output()
        .unwrap();
    let text = render(&output);
    if output.status.success() || !text.contains("error[E6007]") {
        failures.push(format!("E6007 (unknown emit kind): expected E6007\n{text}"));
    }

    // E6004: a sysroot location that carries no manifest.
    let empty = work.join("empty-sysroot");
    fs::create_dir_all(&empty).unwrap();
    let output = Command::new(cli)
        .args(["build-sysroot", "--quiet"])
        .env("LUNA_SYSROOT", &empty)
        .output()
        .unwrap();
    let text = render(&output);
    if output.status.success() || !text.contains("error[E6004]") {
        failures.push(format!("E6004 (sysroot without manifest): expected E6004\n{text}"));
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// A compile failure under `run` keeps the compiler's typed diagnostics; it must
/// not be reclassified as a process execution failure.
#[test]
fn run_keeps_compiler_diagnostics_instead_of_reclassifying_them() {
    let work = fixture_dir();
    let program = work.join("broken.ln");
    fs::write(&program, "fn main() -> i32 {\n    return missing_symbol;\n}\n").unwrap();
    let sysroot = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("run")
        .arg(&program)
        .args(["--quiet", "-I"])
        .arg(&sysroot)
        .env("LUNA_SYSROOT", &sysroot)
        .output()
        .unwrap();
    let text = render(&output);
    assert!(!output.status.success(), "{text}");
    assert!(text.contains("error[E1001]"), "expected the compiler's typed code\n{text}");
    assert!(!text.contains("error[E6009]"), "a compile failure must not be E6009\n{text}");
}
