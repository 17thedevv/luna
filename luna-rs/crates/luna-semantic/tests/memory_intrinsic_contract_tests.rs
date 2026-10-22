use luna_ast::AstArena;
use luna_common::{diagnostic::DiagnosticCode, source::SourceManager};
use luna_semantic::{lang_item::LangItem, Resolver, SemanticContext, TypeChecker};

fn check(source: &str, trusted: bool) -> SemanticContext {
    check_bits(source, trusted, 64)
}

fn check_bits(source: &str, trusted: bool, bits: u32) -> SemanticContext {
    let mut sources = SourceManager::new();
    let file = sources.add_file("memory_contract.ln".into(), source.into());
    let mut arena = AstArena::new();
    let lexer = luna_lexer::Lexer::new(source, file);
    let mut parser = luna_parser::Parser::new(lexer, &mut arena, file);
    let items = parser.parse_file().unwrap();
    assert!(parser.diagnostics.is_empty(), "{:?}", parser.diagnostics);
    let mut ctx = SemanticContext::new();
    ctx.target_pointer_bits = bits;
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
        #[lang("slice_from_raw_parts")] unsafe intrinsic fn view<T>(p: *T, n: usize) -> &[T];
        #[lang("slice_from_raw_parts_mut")] unsafe intrinsic fn view_mut<T>(p: *rw T, n: usize) -> &rw [T];
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
                .any(|d| d.code == Some(DiagnosticCode::InvalidAttributeApplication)),
            "accepted {source}: {:?}",
            ctx.diagnostics
        );
    }
    for declaration in [
        "unsafe intrinsic fn bad<T>(p: *rw T, n: usize) -> &[T];",
        "unsafe intrinsic fn bad<T>(p: *T, n: i32) -> &[T];",
        "unsafe intrinsic fn bad<T>(p: *T, n: usize) -> &rw [T];",
        "unsafe intrinsic fn bad<T>(p: *T, n: usize) -> &[i32];",
        "unsafe intrinsic fn bad<T>(p: *T) -> &[T];",
    ] {
        let source = format!("#[lang(\"slice_from_raw_parts\")] {declaration}");
        let ctx = check(&source, true);
        assert!(
            ctx.diagnostics
                .iter()
                .any(|d| d.code == Some(DiagnosticCode::InvalidAttributeApplication)),
            "accepted {source}: {:?}",
            ctx.diagnostics
        );
    }
}

#[test]
fn canonical_slice_count_is_target_sized_not_same_width_u64() {
    for bits in [32, 64] {
        for (hook, pointer, output) in [
            ("slice_from_raw_parts", "*T", "&[T]"),
            ("slice_from_raw_parts_mut", "*rw T", "&rw [T]"),
        ] {
            for count in ["usize", "u64", "u32", "isize"] {
                let source = format!("#[lang(\"{hook}\")] unsafe intrinsic fn custom<T>(p: {pointer}, count: {count}) -> {output};");
                let ctx = check_bits(&source, true, bits);
                if count == "usize" {
                    assert!(ctx.diagnostics.is_empty(), "{bits} {source}: {:?}", ctx.diagnostics);
                } else {
                    assert!(ctx.diagnostics.iter().any(|d| d.code == Some(DiagnosticCode::InvalidAttributeApplication)),
                        "accepted {bits} {source}: {:?}", ctx.diagnostics);
                }
            }
        }
    }
}

#[test]
fn untrusted_hooks_and_wrong_intrinsic_arity_are_rejected() {
    let source = r#"#[lang("drop_in_place")] unsafe intrinsic fn bad<T>(p: *rw T);"#;
    let ctx = check(source, false);
    assert!(ctx
        .diagnostics
        .iter()
        .any(|d| d.code == Some(DiagnosticCode::InvalidCompilerBinding)));
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
