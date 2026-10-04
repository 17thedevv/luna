use luna_ast::{AstArena, Decl, Item};
use luna_common::{ids::FileId, DiagnosticCode};
use luna_lexer::Lexer;
use luna_parser::Parser;

#[test]
fn namespace_opening_and_alias_have_distinct_ast_identity() {
    let source = "using outer::math; using outer::math as m;";
    let mut arena = AstArena::default();
    let tokens = Lexer::new(source, FileId(0)).collect();
    let mut parser = Parser::from_tokens(tokens, source, None, &mut arena, FileId(0));
    let items = parser.parse_file().unwrap();
    assert!(parser.diagnostics.is_empty());
    let Item::Decl(open) = items[0] else {
        panic!("expected declaration");
    };
    let Item::Decl(alias) = items[1] else {
        panic!("expected declaration");
    };
    let Decl::UsingNamespace { path, span } = &arena.decls[open.0 as usize] else {
        panic!("expected opening");
    };
    assert_eq!(path.len(), 2);
    assert_eq!(
        &source[span.start as usize..span.end as usize],
        "using outer::math;"
    );
    let Decl::Using { path, alias, .. } = &arena.decls[alias.0 as usize] else {
        panic!("expected alias");
    };
    assert_eq!(path.len(), 2);
    assert_eq!(&source[alias.start as usize..alias.end as usize], "m");
}

#[test]
fn exported_using_remains_a_source_syntax_error() {
    for source in ["export using math;", "export using math as m;"] {
        let mut arena = AstArena::default();
        let tokens = Lexer::new(source, FileId(0)).collect();
        let mut parser = Parser::from_tokens(tokens, source, None, &mut arena, FileId(0));
        let _ = parser.parse_file();
        assert!(parser
            .diagnostics
            .iter()
            .any(|d| d.code == Some(DiagnosticCode::InvalidSyntax) && d.span.is_some()));
    }
}
