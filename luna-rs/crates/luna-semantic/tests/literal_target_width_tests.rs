use luna_ast::AstArena;
use luna_common::source::SourceManager;
use luna_lexer::Lexer;
use luna_parser::Parser;
use luna_semantic::{resolver::Resolver, typechecker::TypeChecker, SemanticContext};

fn diagnostics(source: &str, bits: u32) -> Vec<String> {
    let mut manager = SourceManager::new();
    let file = manager.add_file("target_width.ln".into(), source.into());
    let mut arena = AstArena::new();
    let items = Parser::new(Lexer::new(source, file), &mut arena, file)
        .parse_file()
        .unwrap();
    let mut context = SemanticContext::new();
    context.target_pointer_bits = bits;
    Resolver::new(&mut context, &arena, &manager).resolve_items(&items);
    TypeChecker::new(&mut context, &arena, &manager).typecheck_items(&items);
    context.diagnostics.into_iter().map(|d| d.message).collect()
}
#[test]
fn target_sized_literals_use_the_selected_width_not_host_magnitude() {
    for bits in [32, 64] {
        let result = diagnostics(
            "fn limits() { dec U: usize = 4294967295usize; dec I: isize = -2147483648isize; }",
            bits,
        );
        assert!(result.is_empty(), "{bits}: {result:?}");
    }
    for source in [
        "fn limits() { dec U: usize = 4294967296usize; }",
        "fn limits() { dec I: isize = 2147483648isize; }",
    ] {
        assert!(diagnostics(source, 32)
            .iter()
            .any(|d| d.contains("E_INTEGER_LITERAL_RANGE")));
        assert!(diagnostics(source, 64).is_empty());
    }
}
