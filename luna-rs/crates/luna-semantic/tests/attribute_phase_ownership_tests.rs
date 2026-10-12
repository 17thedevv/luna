use luna_ast::AstArena;
use luna_common::{Diagnostic, DiagnosticCode as Code, source::SourceManager};
use luna_common::diagnostic::DiagnosticPhase;
use luna_semantic::{AttributeProcessor, Resolver, SemanticContext, TypeChecker};

fn admit(source: &str, trusted: bool) -> (DiagnosticPhase, Vec<Diagnostic>) {
    let mut sources = SourceManager::new();
    let file = sources.add_file("ownership.ln".into(), source.into());
    let mut arena = AstArena::new();
    let mut parser = luna_parser::Parser::new(luna_lexer::Lexer::new(source, file), &mut arena, file);
    let items = parser.parse_file().unwrap();
    if !parser.diagnostics.is_empty() {
        return (DiagnosticPhase::Parser, parser.diagnostics);
    }
    // Transformation itself cannot emit coded contextual diagnostics.
    let items = match AttributeProcessor::new(&mut arena, &mut sources, file).process_items(items) {
        Ok(items) => items,
        Err(failures) => return (DiagnosticPhase::TypeChecker,
            TypeChecker::admit_attribute_expansion_failures(failures)),
    };
    let mut ctx = SemanticContext::new();
    ctx.allow_internal_lang_items = trusted;
    Resolver::new(&mut ctx, &arena, &sources).register_macros(&items);
    let mut macros = luna_semantic::MacroEngine::new(&mut arena, &sources, file, &ctx.symbol_table, &ctx.tables);
    let items = match macros.expand_items(items) {
        Ok(items) => items,
        Err(failures) => return (DiagnosticPhase::Resolver, Resolver::admit_macro_expansion_failures(failures)),
    };
    Resolver::new(&mut ctx, &arena, &sources).resolve_items(&items);
    if !ctx.diagnostics.is_empty() {
        return (DiagnosticPhase::Resolver, ctx.diagnostics);
    }
    TypeChecker::new(&mut ctx, &arena, &sources).typecheck_items(&items);
    (DiagnosticPhase::TypeChecker, ctx.diagnostics)
}

#[test]
fn diagnostics_originate_in_the_owning_admission_phase_with_original_spans() {
    for (source, trusted, phase, code) in [
        ("#[unknown] fn f() {}", false, DiagnosticPhase::Parser, Code::InvalidAnnotation),
        ("fn f() { dec value = 0b2; }", false, DiagnosticPhase::Parser, Code::InvalidSyntax),
        ("macro m { () => { 1 } } fn f() { dec value = m; }", false, DiagnosticPhase::TypeChecker, Code::TypeMismatch),
        ("macro m { () => { 1 } } fn f() { dec value = m!(1); }", false, DiagnosticPhase::Resolver, Code::InvalidMacroExpansion),
        ("macro m { () => { m!() } } fn f() { dec value = m!(); }", false, DiagnosticPhase::Resolver, Code::InvalidMacroExpansion),
        ("#[link(] fn f() {}", false, DiagnosticPhase::Parser, Code::InvalidAnnotation),
        ("fn __luna_intrinsic_WrappingAdd() {}", false, DiagnosticPhase::Resolver, Code::InvalidCompilerBinding),
        ("#[link(\"__luna_intrinsic_WrappingAdd\")] extern fn f();", false, DiagnosticPhase::Resolver, Code::InvalidCompilerBinding),
        ("#[lang(\"drop_in_place\")] unsafe intrinsic fn f<T>(p: *rw T);", false, DiagnosticPhase::Resolver, Code::InvalidCompilerBinding),
        ("#[lang(\"drop_in_place\")] unsafe intrinsic fn f<T>(p: *T);", true, DiagnosticPhase::TypeChecker, Code::InvalidAttributeApplication),
        ("#[link(\"entry\")] fn main() -> i32 { return 0; }", false, DiagnosticPhase::TypeChecker, Code::InvalidAttributeApplication),
        ("#[test] struct S {};", false, DiagnosticPhase::TypeChecker, Code::InvalidAttributeApplication),
        ("#[derive(Unknown)] struct S {};", false, DiagnosticPhase::TypeChecker, Code::InvalidAttributeApplication),
    ] {
        let (actual_phase, diagnostics) = admit(source, trusted);
        assert_eq!(actual_phase, phase, "{source}: {diagnostics:?}");
        assert!(!diagnostics.is_empty(), "{source}");
        for diagnostic in diagnostics {
            assert_eq!(diagnostic.code, Some(code), "{source}: {diagnostic:?}");
            assert_eq!(code.owner(), phase);
            let span = diagnostic.span.expect("original primary source span");
            assert!(span.start < span.end && span.end as usize <= source.len(), "{diagnostic:?}");
        }
    }
}

#[test]
fn existing_valid_bindings_and_applications_remain_admitted() {
    for (source, trusted) in [
        ("#[link(\"helper_symbol\")] fn helper() -> i32 { return 0; }", false),
        ("module helpers { #[link(\"nested_main_helper\")] fn main(value:i32)->i32 { return value; } }", false),
        ("#[lang(\"drop_in_place\")] unsafe intrinsic fn destroy<T>(p: *rw T);", true),
        ("extern fn f(#[sync_noescape] p: *rw i32);", false),
    ] {
        assert!(admit(source, trusted).1.is_empty(), "{source}");
    }
}

#[test]
fn semantic_attribute_sources_cannot_directly_construct_parser_annotation_codes() {
    for source in [
        include_str!("../src/annotation.rs"), include_str!("../src/derive.rs"),
        include_str!("../src/resolver.rs"), include_str!("../src/lang_item.rs"),
        include_str!("../src/typechecker.rs"), include_str!("../src/attribute_application.rs"),
    ] {
        assert!(!source.contains("DiagnosticCode::InvalidAnnotation"));
    }
    for source in [include_str!("../src/resolver.rs"), include_str!("../src/macro_engine.rs"),
                   include_str!("../src/typechecker.rs")] {
        assert!(!source.contains("DiagnosticCode::InvalidSyntax"), "Parser grammar admission must own E0003");
    }
    assert!(!include_str!("../src/macro_engine.rs").contains("DiagnosticCode::InvalidMacroExpansion"),
        "Resolver admission must construct E1010 from structured expansion failures");
    assert!(std::panic::catch_unwind(|| {
        Diagnostic::error("wrong owner").with_code(Code::InvalidAnnotation)
            .in_phase(DiagnosticPhase::Resolver)
    }).is_err(), "DIAG-8 must fail closed on cross-phase construction");
}

#[test]
fn comptime_statement_rejection_is_owned_by_comptime_and_has_a_source_span() {
    for source in ["fn f() { comptime { 1i32 + 1i32; } }", "fn f() { comptime {} }"] {
        let diagnostics = admit(source, false).1;
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].code, Some(Code::ComptimeEvaluationFailed));
        assert_eq!(Code::ComptimeEvaluationFailed.owner(), DiagnosticPhase::AsyncComptime);
        let span = diagnostics[0].span.unwrap();
        assert!(span.start < span.end && span.end as usize <= source.len());
    }
}
