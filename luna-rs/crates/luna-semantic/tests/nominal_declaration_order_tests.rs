use luna_ast::AstArena;
use luna_common::source::SourceManager;
use luna_lexer::Lexer;
use luna_parser::Parser;
use luna_semantic::{SemanticContext, resolver::Resolver, typechecker::TypeChecker};

fn diagnostics(source: &str) -> Vec<String> {
    let mut manager = SourceManager::new();
    let file = manager.add_file("nominal_order.ln".into(), source.into());
    let mut arena = AstArena::new();
    let items = Parser::new(Lexer::new(source, file), &mut arena, file)
        .parse_file()
        .expect("fixture must parse");
    let mut context = SemanticContext::new();
    Resolver::new(&mut context, &arena, &manager).resolve_items(&items);
    TypeChecker::new(&mut context, &arena, &manager).typecheck_items(&items);
    context.diagnostics.into_iter().map(|d| d.message).collect()
}

#[test]
fn forward_struct_fields_have_the_same_types_as_ordered_declarations() {
    for source in [
        include_str!("../../../tests/luna/compiler/forward_nominal_type_probe.ln"),
        include_str!("../../../tests/luna/compiler/forward_nominal_ordered_control.ln"),
        include_str!("../../../tests/luna/compiler/forward_nominal_unused_probe.ln"),
    ] {
        let errors = diagnostics(source);
        assert!(errors.is_empty(), "{errors:?}");
    }
}

#[test]
fn forward_generic_nominal_preserves_concrete_field_substitution() {
    let errors = diagnostics(
        "struct Early { item: Later<u8>, };\n\
         struct Later<T> { value: T, };\n\
         fn read(x: Early) -> u8 { return x.item.value; }\n\
         fn main() -> i32 { dec x = Early { item: Later<u8> { value: 7 } }; return read(x) as i32 - 7; }",
    );
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn forward_nominal_population_uses_the_declarations_lexical_scope() {
    let errors = diagnostics(
        "module scoped {\n\
         struct Early { item: Later, };\n\
         struct Later { value: i32, };\n\
         fn read(x: Early) -> i32 { return x.item.value; }\n\
         }\n\
         struct Later { other: bool, };\n\
         fn main() -> i32 { return 0; }",
    );
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn forward_field_population_does_not_weaken_types_or_repeat_diagnostics() {
    let errors = diagnostics(
        "struct Early { item: Later, };\n\
         struct Later { value: u8, };\n\
         fn main() { dec value: i32 = 7; dec x = Early { item: Later { value: value } }; }",
    );
    assert!(
        errors.iter().any(|error| error.contains("mismatch")),
        "{errors:?}"
    );
    let errors = diagnostics(
        "struct Early { item: Later, };\n\
         struct Later { value: i32, value: i32, };\n\
         fn main() {}",
    );
    assert_eq!(
        errors
            .iter()
            .filter(|error| error.contains("Duplicate field"))
            .count(),
        1,
        "{errors:?}"
    );
    let errors = diagnostics("struct Early { item: Missing, }; fn main() {}");
    assert!(
        !errors.is_empty(),
        "unknown nominal type must remain rejected"
    );
}
