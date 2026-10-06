use luna_ast::{AstArena, AstRelocator, Expr, MacroDelimiter, Pattern, TokenTree, Type};
use luna_common::ids::{FileId, Span};
use luna_lexer::{Token, TokenKind};

fn token() -> Token {
    Token::new(TokenKind::IntegerLiteral, Span::new(FileId(0), 7, 10))
}

fn relocate(arena: &mut AstArena) {
    AstRelocator::new(11, 22, 33, 44, 55, FileId(9)).relocate_arena(arena);
}

fn check(span: Span) {
    assert_eq!(
        span.file_id,
        FileId(9),
        "producer FileId must not name the consumer source"
    );
    assert_eq!(
        (span.start, span.end),
        (7, 10),
        "offsets remain relative to embedded provider source"
    );
}

#[test]
fn literal_and_literal_pattern_and_try_diagnostics_relocate() {
    let mut arena = AstArena::new();
    let literal = arena.alloc_expr(Expr::Literal(token(), "256u8".into()));
    arena.alloc_expr(Expr::Try {
        expr: literal,
        try_span: token().span,
    });
    arena.alloc_pat(Pattern::Literal(token()));
    relocate(&mut arena);
    let Expr::Literal(token, text) = &arena.exprs[0] else {
        panic!()
    };
    check(token.span);
    assert_eq!(text, "256u8");
    let Expr::Try { expr, try_span } = arena.exprs[1] else {
        panic!()
    };
    assert_eq!(expr.0, 11);
    check(try_span);
    let Pattern::Literal(token) = arena.pats[0] else {
        panic!()
    };
    check(token.span);
}

#[test]
fn parameter_default_expression_and_contract_span_relocate() {
    let mut arena = AstArena::new();
    let value = arena.alloc_expr(Expr::Literal(token(), "3".into()));
    arena.alloc_decl(luna_ast::Decl::Param {
        annotations: Vec::new(), visibility: luna_ast::Visibility::Private,
        name: token().span, ty: None, is_self: false, is_variadic: false,
        default: Some(luna_ast::ParamDefault { value, span: token().span }),
    });
    relocate(&mut arena);
    let luna_ast::Decl::Param { default: Some(default), .. } = &arena.decls[0] else { panic!() };
    assert_eq!(default.value.0, 11);
    check(default.span);
}

fn tree() -> TokenTree {
    TokenTree::Group {
        delimiter: MacroDelimiter::Paren,
        span: token().span,
        tokens: vec![TokenTree::Leaf { token: token() }],
    }
}

fn check_tree(tree: &TokenTree) {
    match tree {
        TokenTree::Leaf { token } => check(token.span),
        TokenTree::Group { span, tokens, .. } => {
            check(*span);
            for child in tokens {
                check_tree(child);
            }
        }
    }
}

#[test]
fn expression_and_type_macro_tokens_relocate_recursively() {
    let mut arena = AstArena::new();
    arena.alloc_expr(Expr::MacroCall {
        name: token().span,
        path: vec![token().span],
        delimiter: MacroDelimiter::Paren,
        args: vec![tree()],
        raw_tokens: vec![token()],
        span: token().span,
    });
    arena.alloc_type(Type::MacroCall {
        name: token().span,
        path: vec![token().span],
        delimiter: MacroDelimiter::Paren,
        args: vec![tree()],
        raw_tokens: vec![token()],
        span: token().span,
    });
    relocate(&mut arena);
    let Expr::MacroCall {
        name,
        path,
        args,
        raw_tokens,
        span,
        ..
    } = &arena.exprs[0]
    else {
        panic!()
    };
    check(*name);
    check(*span);
    for span in path {
        check(*span);
    }
    for tree in args {
        check_tree(tree);
    }
    for token in raw_tokens {
        check(token.span);
    }
    let Type::MacroCall {
        name,
        path,
        args,
        raw_tokens,
        span,
        ..
    } = &arena.types[0]
    else {
        panic!()
    };
    check(*name);
    check(*span);
    for span in path {
        check(*span);
    }
    for tree in args {
        check_tree(tree);
    }
    for token in raw_tokens {
        check(token.span);
    }
}
