use luna_ast::{AstArena, Expr};
use luna_common::ids::FileId;
use luna_lexer::{Lexer, TokenKind};
use luna_parser::Parser;

#[test]
fn numeric_suffix_and_byte_literals_preserve_source_tokens() {
    let source =
        "fn main() -> i32 { dec a = 0xffu8; dec b = b'A'; dec c = b\"A\\0\\xFF\"; return 0; }";
    let mut arena = AstArena::default();
    let tokens = Lexer::new(source, FileId(0)).collect();
    let mut parser = Parser::from_tokens(tokens, source, None, &mut arena, FileId(0));
    assert!(parser.parse_file().is_ok(), "{:?}", parser.diagnostics);
    assert!(parser.diagnostics.is_empty(), "{:?}", parser.diagnostics);
    for (kind, spelling) in [
        (TokenKind::IntegerLiteral, "0xffu8"),
        (TokenKind::ByteLiteral, "b'A'"),
        (TokenKind::ByteStringLiteral, "b\"A\\0\\xFF\""),
    ] {
        assert!(arena.exprs.iter().any(|expr| matches!(expr, Expr::Literal(token, text) if token.kind == kind && text == spelling)));
    }
}
