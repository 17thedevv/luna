use luna_ast::AstArena;
use luna_common::{diagnostic::DiagnosticCode, source::SourceManager};
use luna_semantic::{lang_item::LangItem, Resolver, SemanticContext, TypeChecker};

fn check(source: &str, trusted: bool) -> SemanticContext {
    let mut sources = SourceManager::new();
    let file = sources.add_file("memory_contract.ln".into(), source.into());
    let mut arena = AstArena::new();
    let lexer = luna_lexer::Lexer::new(source, file);
    let mut parser = luna_parser::Parser::new(lexer, &mut arena, file);
    let items = parser.parse_file().unwrap();
    assert!(parser.diagnostics.is_empty(), "{:?}", parser.diagnostics);
    let mut ctx = SemanticContext::new();
    ctx.allow_internal_lang_items = trusted;
    Resolver::new(&mut ctx, &arena, &sources).resolve_items(&items);
    TypeChecker::new(&mut ctx, &arena, &sources).typecheck_items(&items);
    ctx
}

#[test]
fn renamed_trusted_declarations_keep_typed_intrinsic_identity() {
    let ctx = check(
        r#"
        #[lang("drop_in_place")] unsafe intrinsic fn destroy<T>(p: *rw T);
        #[lang("slice_from_raw_parts")] unsafe intrinsic fn view<T>(p: *T, n: u64) -> &[T];
        #[lang("slice_from_raw_parts_mut")] unsafe intrinsic fn view_mut<T>(p: *rw T, n: u64) -> &rw [T];
    "#,
        true,
    );
    assert!(ctx.diagnostics.is_empty(), "{:?}", ctx.diagnostics);
    for (item, name) in [
        (LangItem::DropInPlace, "destroy"),
        (LangItem::SliceFromRawParts, "view"),
        (LangItem::SliceFromRawPartsMut, "view_mut"),
    ] {
        let symbol = ctx.lang_items.get(item).unwrap();
        assert_eq!(ctx.symbol_table.get_symbol(symbol).name, name);
    }
}

#[test]
fn malformed_trusted_intrinsic_signatures_fail_closed() {
    for declaration in [
        "unsafe fn bad<T>(p: *rw T);",
        "intrinsic fn bad<T>(p: *rw T);",
        "unsafe intrinsic fn bad<T>(p: *T);",
        "unsafe intrinsic fn bad<T>(p: *rw i32);",
        "unsafe intrinsic fn bad<T,U>(p: *rw T);",
        "unsafe intrinsic fn bad<T>(p: *rw T) -> i32;",
        "unsafe intrinsic fn bad<T>();",
        "unsafe intrinsic fn bad<T>(p: *rw T) {}",
    ] {
        let source = format!("#[lang(\"drop_in_place\")] {declaration}");
        let ctx = check(&source, true);
        assert!(
            ctx.diagnostics
                .iter()
                .any(|d| d.code == Some(DiagnosticCode::InvalidAnnotation)),
            "accepted {source}: {:?}",
            ctx.diagnostics
        );
    }
    for declaration in [
        "unsafe intrinsic fn bad<T>(p: *rw T, n: u64) -> &[T];",
        "unsafe intrinsic fn bad<T>(p: *T, n: i32) -> &[T];",
        "unsafe intrinsic fn bad<T>(p: *T, n: u64) -> &rw [T];",
        "unsafe intrinsic fn bad<T>(p: *T, n: u64) -> &[i32];",
        "unsafe intrinsic fn bad<T>(p: *T) -> &[T];",
    ] {
        let source = format!("#[lang(\"slice_from_raw_parts\")] {declaration}");
        let ctx = check(&source, true);
        assert!(
            ctx.diagnostics
                .iter()
                .any(|d| d.code == Some(DiagnosticCode::InvalidAnnotation)),
            "accepted {source}: {:?}",
            ctx.diagnostics
        );
    }
}

#[test]
fn untrusted_hooks_and_wrong_intrinsic_arity_are_rejected() {
    let source = r#"#[lang("drop_in_place")] unsafe intrinsic fn bad<T>(p: *rw T);"#;
    let ctx = check(source, false);
    assert!(ctx
        .diagnostics
        .iter()
        .any(|d| d.code == Some(DiagnosticCode::InvalidAnnotation)));
    for call in ["destroy<i32>()", "destroy<i32>(0 as *rw i32, 1)"] {
        let ctx = check(
            &format!(
                r#"
            #[lang("drop_in_place")] unsafe intrinsic fn destroy<T>(p: *rw T);
            fn test() {{ unsafe {{ {call}; }} }}
        "#
            ),
            true,
        );
        assert!(
            ctx.diagnostics
                .iter()
                .any(|d| d.code == Some(DiagnosticCode::TypeMismatch)),
            "accepted {call}: {:?}",
            ctx.diagnostics
        );
    }
}
