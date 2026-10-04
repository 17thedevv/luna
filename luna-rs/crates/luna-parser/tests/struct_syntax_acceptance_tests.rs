use luna_ast::{AstArena, Decl, Item};
use luna_common::ids::FileId;
use luna_common::Diagnostic;
use luna_lexer::Lexer;
use luna_parser::Parser;

fn parse(input: &str) -> (Result<Vec<Item>, ()>, AstArena, Vec<Diagnostic>) {
    let mut arena = AstArena::default();
    let file_id = FileId(0);
    let lexer = Lexer::new(input, file_id);
    let mut parser = Parser::new(lexer, &mut arena, file_id);
    let result = parser.parse_file();
    let diags = parser.diagnostics;
    (result, arena, diags)
}

/// STRUCT-SYNTAX-01: struct A {}; -> accepted
#[test]
fn test_struct_syntax_01_empty_struct_with_semicolon() {
    let input = "struct A {};";
    let (result, arena, diags) = parse(input);
    assert!(diags.is_empty(), "Expected no diagnostics, got: {:?}", diags);
    let items = result.expect("parse successful");
    assert_eq!(items.len(), 1);
    if let Item::Decl(decl_id) = items[0] {
        if let Decl::Struct { name, fields, lifetime_contract, .. } = &arena.decls[decl_id.0 as usize] {
            assert_eq!(name.start, 7);
            assert!(fields.is_empty());
            assert!(lifetime_contract.is_none());
        } else {
            panic!("Expected Decl::Struct");
        }
    } else {
        panic!("Expected Item::Decl");
    }
}

/// STRUCT-SYNTAX-02: struct Holder { value: &i32 } requires life(value) >= life(self); -> accepted
#[test]
fn test_struct_syntax_02_struct_with_postfix_lifetime_contract() {
    let input = "struct Holder { value: &i32, } requires life(value) >= life(self);";
    let (result, arena, diags) = parse(input);
    assert!(diags.is_empty(), "Expected no diagnostics, got: {:?}", diags);
    let items = result.expect("parse successful");
    assert_eq!(items.len(), 1);
    if let Item::Decl(decl_id) = items[0] {
        if let Decl::Struct { fields, lifetime_contract, .. } = &arena.decls[decl_id.0 as usize] {
            assert_eq!(fields.len(), 1);
            let contract = lifetime_contract.as_ref().expect("Expected lifetime_contract");
            assert_eq!(contract.constraints.len(), 1);
            assert_eq!(contract.constraints[0].longer.to_string(), "value");
            assert_eq!(contract.constraints[0].shorter.to_string(), "self");
        } else {
            panic!("Expected Decl::Struct");
        }
    } else {
        panic!("Expected Item::Decl");
    }
}

/// STRUCT-SYNTAX-03: struct Holder { value: &i32 }; requires life(value) >= life(self); -> rejected
/// because the first ';' terminates the struct declaration, leaving 'requires' invalid at top level.
#[test]
fn test_struct_syntax_03_semicolon_before_requires_rejected() {
    let input = "struct Holder { value: &i32, }; requires life(value) >= life(self);";
    let (result, _arena, diags) = parse(input);
    assert!(
        result.is_err() || !diags.is_empty(),
        "Expected syntax error when ';' precedes 'requires', but parsing succeeded with no diagnostics"
    );
}

/// STRUCT-SYNTAX-04: struct A {} -> rejected: expected ';' after struct declaration
#[test]
fn test_struct_syntax_04_missing_semicolon_rejected() {
    let input = "struct A {}";
    let (result, _arena, diags) = parse(input);
    assert!(
        result.is_err() || !diags.is_empty(),
        "Expected syntax error for missing semicolon after struct declaration"
    );
    assert!(
        diags.iter().any(|d| d.message.contains("Expected ';'") || d.message.contains("Expected ';' after struct declaration")),
        "Expected error message mentioning missing ';', got: {:?}",
        diags
    );
}

/// STRUCT-SYNTAX-05: struct A {};; -> second ';' rejected normally / treated according to empty-statement policy
#[test]
fn test_struct_syntax_05_double_semicolon_rejected() {
    let input = "struct A {};;";
    let (result, _arena, diags) = parse(input);
    assert!(
        result.is_err() || !diags.is_empty(),
        "Expected syntax error for redundant second semicolon, but got success with no diagnostics"
    );
}

/// RAW-STORAGE-ANCHOR-v1: direct raw pointer field syntax and canonical name
/// survive parsing independently of ordinary lifetime clauses.
#[test]
fn test_struct_raw_storage_anchor_syntax() {
    let input = "struct RawOwner { data: *rw u8, } requires anchor(data) = self;";
    let (result, arena, diags) = parse(input);
    assert!(diags.is_empty(), "Expected no diagnostics, got: {:?}", diags);
    let items = result.expect("parse successful");
    let Item::Decl(decl_id) = items[0] else { panic!("Expected struct declaration") };
    let Decl::Struct { raw_storage_anchor_contract, lifetime_contract, .. } = &arena.decls[decl_id.0 as usize] else {
        panic!("Expected Decl::Struct");
    };
    assert!(lifetime_contract.is_none());
    let contract = raw_storage_anchor_contract.as_ref().expect("expected raw anchor contract");
    assert_eq!(contract.anchors.len(), 1);
    assert_eq!(contract.anchors[0].field_name, "data");
}

#[test]
fn test_struct_lifetime_and_raw_anchor_clauses_coexist() {
    let input = "struct Owner { borrowed: &i32, data: *rw u8, } requires life(borrowed) >= life(self) requires anchor(data) = self;";
    let (result, arena, diags) = parse(input);
    assert!(diags.is_empty(), "Expected no diagnostics, got: {:?}", diags);
    let items = result.expect("parse successful");
    let Item::Decl(decl_id) = items[0] else { panic!("Expected struct declaration") };
    let Decl::Struct { raw_storage_anchor_contract, lifetime_contract, .. } = &arena.decls[decl_id.0 as usize] else {
        panic!("Expected Decl::Struct");
    };
    assert_eq!(lifetime_contract.as_ref().unwrap().constraints.len(), 1);
    assert_eq!(raw_storage_anchor_contract.as_ref().unwrap().anchors[0].field_name, "data");
}

#[test]
fn test_struct_raw_storage_anchor_rejects_nested_path_syntax() {
    let input = "struct RawOwner { data: *rw u8, } requires anchor(nested.data) = self;";
    let (result, _arena, diags) = parse(input);
    assert!(result.is_err() || !diags.is_empty(), "nested anchor paths are not in v1 grammar");
}

#[test]
fn struct_requires_accepts_multiple_anchors_in_one_group() {
    let input = r#"
export struct TreeStorage<K, V> {
    private keys: *rw K,
    private values: *rw V,
    private lefts: *rw u64,
    private rights: *rw u64,
    private heights: *rw i32,
} requires anchor(keys) = self,
           anchor(values) = self,
           anchor(lefts) = self,
           anchor(rights) = self,
           anchor(heights) = self;
"#;
    let (result, arena, diags) = parse(input);
    assert!(diags.is_empty(), "{diags:?}");
    let items = result.expect("parse successful");
    assert_eq!(items.len(), 1);
    let Item::Decl(decl_id) = items[0] else { panic!("Expected struct declaration") };
    let Decl::Struct { raw_storage_anchor_contract, lifetime_contract, generic_params, fields, .. } = &arena.decls[decl_id.0 as usize] else {
        panic!("Expected Decl::Struct");
    };
    assert_eq!(generic_params.len(), 2);
    assert_eq!(fields.len(), 5);
    assert!(lifetime_contract.is_none());
    let anchors = &raw_storage_anchor_contract.as_ref().unwrap().anchors;
    assert_eq!(
        anchors.iter().map(|anchor| anchor.field_name.as_str()).collect::<Vec<_>>(),
        ["keys", "values", "lefts", "rights", "heights"]
    );
    for anchor in anchors {
        assert_eq!(
            &input[anchor.span.start as usize..anchor.span.end as usize],
            anchor.field_name
        );
    }
}

#[test]
fn struct_requires_accepts_mixed_contract_entries_in_either_order() {
    for contracts in [
        "anchor(data) = self, life(borrowed) >= life(self), anchor(other) = self",
        "life(borrowed) >= life(self), anchor(data) = self, anchor(other) = self",
        "anchor(data) = self, anchor(other) = self, life(self) <= life(borrowed)",
    ] {
        let input = format!(
            "struct Owner {{ borrowed: &i32, data: *rw u8, other: *u16, }} requires {contracts};"
        );
        let (result, arena, diags) = parse(&input);
        assert!(diags.is_empty(), "{contracts}: {diags:?}");
        let items = result.expect("parse successful");
        let Item::Decl(decl_id) = items[0] else { panic!("Expected struct declaration") };
        let Decl::Struct { raw_storage_anchor_contract, lifetime_contract, .. } = &arena.decls[decl_id.0 as usize] else {
            panic!("Expected Decl::Struct");
        };
        let constraints = &lifetime_contract.as_ref().unwrap().constraints;
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].longer.to_string(), "borrowed");
        assert_eq!(constraints[0].shorter.to_string(), "self");
        let anchors = &raw_storage_anchor_contract.as_ref().unwrap().anchors;
        assert_eq!(anchors.len(), 2);
        assert_eq!(anchors[0].field_name, "data");
        assert_eq!(anchors[1].field_name, "other");
    }
}

#[test]
fn struct_requires_rejects_malformed_contract_lists() {
    for contracts in [
        "",
        ", anchor(data) = self",
        "anchor(data) = self,",
        "anchor(data) = self,, anchor(other) = self",
        "anchor(data) = self anchor(other) = self",
        "life(borrowed) >= life(self) anchor(data) = self",
        "anchor(data) = self, anchor(other) = owner",
        "anchor(data) = self, anchor(other.ptr) = self",
    ] {
        let input = format!(
            "struct Owner {{ borrowed: &i32, data: *rw u8, other: *u16, }} requires {contracts};"
        );
        let (result, _arena, diags) = parse(&input);
        assert!(result.is_err() || !diags.is_empty(), "accepted malformed contract list: {contracts}");
    }
}

#[test]
fn empty_struct_literals_compose_with_outer_fields_operators_and_tail_expressions() {
    for body in [
        "dec value = Wrapper<Item> { item: Item {} };",
        "dec value = Outer { inner: Inner { item: Item {} } };",
        "dec value = (Item {}, [Item {}, Item {}]);",
        "return Wrapper<Item> { item: Item {} };",
        "Item {}",
        "dec value = Item {} == Item {};",
        "dec value = library::Wrapper<Item> { item: library::Item {} };",
    ] {
        let input = format!("fn example() {{ {body} }}");
        let (result, arena, diags) = parse(&input);
        assert!(result.is_ok() && diags.is_empty(), "{body}: {diags:?}");
        assert!(arena.exprs.iter().any(|expr| matches!(expr, luna_ast::Expr::StructInit { .. })));
    }
}

#[test]
fn empty_condition_blocks_do_not_become_struct_literals() {
    let input = "fn example() { if condition {} while condition {} match value {} }";
    let (result, arena, diags) = parse(input);
    assert!(result.is_ok() && diags.is_empty(), "{diags:?}");
    assert!(!arena.exprs.iter().any(|expr| matches!(expr, luna_ast::Expr::StructInit { .. })));
}
