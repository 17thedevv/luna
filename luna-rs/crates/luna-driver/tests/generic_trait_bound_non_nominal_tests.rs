use luna_driver::{CompilerOptions, check_semantic_only};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn check_fixture(name: &str, source: &str) -> Result<(), String> {
    let work = std::env::temp_dir().join(format!(
        "luna_trait_bound_non_nominal_{}_{}_{}",
        name,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    fs::create_dir_all(&work).expect("create semantic fixture directory");
    let source_path = work.join("main.ln");
    fs::write(&source_path, source).expect("write semantic fixture");
    let options = CompilerOptions {
        quiet: true,
        ..Default::default()
    };

    check_semantic_only(&source_path.to_string_lossy(), source.to_string(), &options)
        .map_err(|diagnostics| {
            diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect::<Vec<_>>()
                .join("\n")
        })
}

#[test]
fn concrete_non_nominal_types_must_satisfy_generic_trait_bounds() {
    let accepted = include_str!("../../../tests/luna/compiler/generic_trait_bound_primitive_accepts.ln");
    check_fixture("primitive_positive", accepted)
        .unwrap_or_else(|error| panic!("i32 has a Supported impl and must pass: {error}"));

    let rejected = include_str!("../../../tests/luna/compiler/generic_trait_bound_array_rejects.ln");
    let diagnostics = check_fixture("array_negative", rejected)
        .expect_err("[u8; 1] has no Supported impl and must fail the generic bound");
    assert!(
        diagnostics.contains("does not implement trait `Supported`"),
        "expected trait-bound diagnostic, got: {diagnostics}"
    );
}
