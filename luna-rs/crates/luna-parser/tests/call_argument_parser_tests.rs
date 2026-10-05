use luna_ast::{AstArena, Decl, Expr, Item};
use luna_common::ids::FileId;

#[test]
fn labels_use_equals_or_colon_only_at_the_outer_argument_boundary() {
    for source in [
        "dec value = f(right=2, left:9);",
        "dec value = object.f(right:2, left=9);",
    ] {
        let mut arena = AstArena::new();
        let file = FileId(0);
        let mut parser =
            luna_parser::Parser::new(luna_lexer::Lexer::new(source, file), &mut arena, file);
        let items = parser.parse_file().unwrap();
        assert!(parser.diagnostics.is_empty(), "{:?}", parser.diagnostics);
        let Item::Decl(declaration) = items[0] else {
            panic!()
        };
        let Decl::Var {
            initializer: Some(value),
            ..
        } = arena.decls[declaration.0 as usize]
        else {
            panic!()
        };
        let (Expr::Call { args, .. } | Expr::MethodCall { args, .. }) =
            &arena.exprs[value.0 as usize]
        else {
            panic!()
        };
        assert_eq!(args.len(), 2);
        assert_eq!(
            args.iter()
                .map(|argument| argument
                    .label
                    .map(|span| &source[span.start as usize..span.end as usize]))
                .collect::<Vec<_>>(),
            [Some("right"), Some("left")]
        );
    }
    let source = "dec value = f((target=2));";
    let mut arena = AstArena::new();
    let file = FileId(0);
    let mut parser =
        luna_parser::Parser::new(luna_lexer::Lexer::new(source, file), &mut arena, file);
    parser.parse_file().unwrap();
    assert!(parser.diagnostics.is_empty());
    let argument = arena
        .exprs
        .iter()
        .find_map(|expr| {
            if let Expr::Call { args, .. } = expr {
                args.first()
            } else {
                None
            }
        })
        .unwrap();
    assert!(argument.label.is_none());
    assert!(matches!(
        arena.exprs[argument.value.0 as usize],
        Expr::Assign { .. }
    ));
}
