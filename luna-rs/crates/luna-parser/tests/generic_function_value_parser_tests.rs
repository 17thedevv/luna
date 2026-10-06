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

#[test]
fn qualified_owner_and_method_arguments_keep_their_declaration_roles() {
    let (mut arena, value) = initializer("dec result = api::Processor<i32>::choose<bool>(true);");
    let Expr::Call { callee, generic_args, .. } = &arena.exprs[value.0 as usize] else { panic!() };
    assert_eq!(generic_args.len(), 1);
    let callee = *callee;
    let method_argument = generic_args[0];
    let Expr::Identifier { segments, owner_generic_args, generic_args } = &arena.exprs[callee.0 as usize] else { panic!() };
    assert_eq!(segments.len(), 3);
    assert!(generic_args.is_empty());
    assert_eq!(owner_generic_args.len(), 1);
    assert_eq!(owner_generic_args[0].0, 1);
    let owner_argument = owner_generic_args[0].1[0];
    assert_ne!(method_argument, owner_argument);
    luna_ast::AstRelocator::new(0, 0, 0, 17, 0, luna_common::ids::FileId(9)).relocate_arena(&mut arena);
    let Expr::Call { generic_args, .. } = &arena.exprs[value.0 as usize] else { panic!() };
    assert_eq!(generic_args[0].0, method_argument.0 + 17);
    let Expr::Identifier { segments, owner_generic_args, .. } = &arena.exprs[callee.0 as usize] else { panic!() };
    assert_eq!(owner_generic_args[0].1[0].0, owner_argument.0 + 17);
    assert!(segments.iter().all(|span| span.file_id == luna_common::ids::FileId(9)));
}
