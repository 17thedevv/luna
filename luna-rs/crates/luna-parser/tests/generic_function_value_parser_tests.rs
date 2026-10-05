use luna_ast::{AstArena, Decl, Expr, Item};
use luna_common::source::SourceManager;

fn initializer(source: &str) -> (AstArena, luna_ast::ExprId) {
    let mut sources = SourceManager::new();
    let file = sources.add_file("generic_value.ln".into(), source.into());
    let mut arena = AstArena::new();
    let lexer = luna_lexer::Lexer::new(source, file);
    let mut parser = luna_parser::Parser::new(lexer, &mut arena, file);
    let items = parser.parse_file().unwrap();
    assert!(parser.diagnostics.is_empty(), "{:?}", parser.diagnostics);
    let Item::Decl(declaration) = items.last().unwrap() else {
        panic!()
    };
    let Decl::Var {
        initializer: Some(value),
        ..
    } = arena.decls[declaration.0 as usize]
    else {
        panic!()
    };
    (arena, value)
}

#[test]
fn named_and_nested_type_arguments_are_recognized_on_function_values() {
    for source in [
        "dec callback = factory<Tracker>;",
        "dec callback = factory<Envelope<Tracker>>;",
        "dec callback = factory<module_name::Tracker>;",
        "dec callback = factory<unsafe fn(i32) -> i32>;",
    ] {
        let (arena, value) = initializer(source);
        assert!(
            matches!(&arena.exprs[value.0 as usize], Expr::Identifier { generic_args, .. } if generic_args.len() == 1)
        );
    }
}

#[test]
fn relational_and_shift_expressions_remain_ordinary_operators() {
    for source in [
        "dec value = a < b;",
        "dec value = a < b >> 1;",
        "dec value = a < b > c;",
    ] {
        let (arena, value) = initializer(source);
        assert!(matches!(
            &arena.exprs[value.0 as usize],
            Expr::Binary { .. }
        ));
    }
}
