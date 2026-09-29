//! C-GAP-12: safe-loan effect inference must not confuse data/raw-pointer
//! provenance with safe loans carried by returned values.

use luna_common::CompilerSession;
use luna_driver::session::DriverSession;
use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{compile, CompilerOptions};
use luna_mvir::MvirGenerator;
use luna_semantic::{AttributeProcessor, MacroEngine, MonoCollector, Resolver, SemanticContext, TypeChecker};
use luna_borrowck::effect::{
    RawPointerAnchorReturnEffect, RawPointerAnchorSource, RawPointerReturnEffect, ReturnEffect,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "luna_c_gap_12_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("create isolated C-GAP-12 directory");
    path
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create isolated directory");
    for entry in fs::read_dir(source).expect("enumerate source directory") {
        let entry = entry.expect("read directory entry");
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(from, to).expect("copy isolated source/artifact");
        }
    }
}

fn compile_source_module(root: &Path, source: &str, source_path: &Path) -> (luna_mvir::Module, SemanticContext) {
    let mut session = CompilerSession::new();
    let file_id = session
        .source_manager
        .add_file(source_path.to_string_lossy().to_string(), source.to_string());
    let lexer = luna_lexer::Lexer::new(source, file_id);
    let mut arena = luna_ast::AstArena::new();
    let mut parser = luna_parser::Parser::new(lexer, &mut arena, file_id);
    let mut items = parser.parse_file().expect("parse C-GAP-12 source");
    assert!(parser.diagnostics.is_empty(), "parser diagnostics: {:?}", parser.diagnostics);

    let search_paths = vec![root.to_string_lossy().to_string()];
    let mut driver = DriverSession::new(
        Sysroot::from_root(root.to_path_buf()).expect("isolated sysroot"),
        &mut session,
        &search_paths,
    );
    driver.bootstrap_lang_contracts(&mut arena).expect("bootstrap language contracts");
    luna_driver::importer::resolve_imports(
        &mut items,
        &mut arena,
        &mut driver,
        luna_driver::resolution_context::ProviderResolutionContext::UserImport,
    )
    .expect("resolve C-GAP-12 imports");
    let registry = std::mem::take(&mut driver.registry);
    drop(driver);

    let mut context = SemanticContext::new();
    let mut attributes = AttributeProcessor::new(&mut arena, &mut session.source_manager, file_id);
    items = attributes.process_items(items).expect("process C-GAP-12 attributes");
    registry.inject_into_ctx(&mut context);
    let mut resolver = Resolver::new(&mut context, &arena, &session.source_manager);
    resolver.register_macros(&items);
    let mut macros = MacroEngine::new(
        &mut arena,
        &session.source_manager,
        file_id,
        &context.symbol_table,
        &context.tables,
    );
    items = macros.expand_items(items).expect("expand C-GAP-12 macros");
    Resolver::new(&mut context, &arena, &session.source_manager).resolve_items(&items);
    let comptime = luna_mvir::MvirComptimeEngine { max_steps: 1_000_000, max_depth: 512 };
    TypeChecker::new_with_engine(&mut context, &arena, &session.source_manager, &comptime)
        .typecheck_items(&items);
    assert!(context.diagnostics.is_empty(), "semantic diagnostics: {:?}", context.diagnostics);

    let mut mono = MonoCollector::new_with_source(&mut context, &arena, Some(&session.source_manager));
    mono.run(&items);
    let instances = std::mem::take(&mut mono.instantiated).into_values().collect();
    let drop_glues = std::mem::take(&mut mono.drop_glues);
    drop(mono);
    context.instantiated_functions = instances;
    context.drop_glue_instances = drop_glues;

    let (module, diagnostics) = MvirGenerator::new(&arena, &context, &session.source_manager).generate(&items);
    assert!(diagnostics.is_empty(), "MVIR diagnostics: {diagnostics:?}");
    (module, context)
}

fn return_effect(
    module: &luna_mvir::Module,
    summaries: &std::collections::HashMap<luna_mvir::GlobalId, luna_borrowck::effect::CallEffectSummary>,
    fragment: &str,
) -> ReturnEffect {
    let function = module
        .functions
        .iter()
        .find(|function| function.name.name.contains(fragment))
        .unwrap_or_else(|| panic!("missing function matching {fragment}"));
    summaries.get(&function.name).expect("inferred function summary").ret.clone()
}

fn raw_pointer_return_effect(
    module: &luna_mvir::Module,
    summaries: &std::collections::HashMap<luna_mvir::GlobalId, luna_borrowck::effect::CallEffectSummary>,
    fragment: &str,
) -> RawPointerReturnEffect {
    let function = module
        .functions
        .iter()
        .find(|function| function.name.name.contains(fragment))
        .unwrap_or_else(|| panic!("missing function matching {fragment}"));
    summaries.get(&function.name).expect("inferred function summary").raw_pointer_ret.clone()
}

fn raw_pointer_anchor_return_effect(
    module: &luna_mvir::Module,
    summaries: &std::collections::HashMap<luna_mvir::GlobalId, luna_borrowck::effect::CallEffectSummary>,
    fragment: &str,
) -> RawPointerAnchorReturnEffect {
    let function = module
        .functions
        .iter()
        .find(|function| function.name.name.contains(fragment))
        .unwrap_or_else(|| panic!("missing function matching {fragment}"));
    summaries.get(&function.name).expect("inferred function summary").raw_pointer_anchor_ret.clone()
}

fn compile_consumer(root: &Path, source: &str, directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).expect("create consumer directory");
    let source_path = directory.join("main.ln");
    let output_path = directory.join(if cfg!(windows) { "main.exe" } else { "main" });
    fs::write(&source_path, source).expect("write consumer source");
    let options = CompilerOptions {
        search_paths: vec![root.to_string_lossy().to_string()],
        output_path: Some(output_path.to_string_lossy().to_string()),
        quiet: true,
        ..Default::default()
    };
    compile(source_path.to_str().unwrap(), source.to_string(), &options)
        .map(|_| ())
        .map_err(|diagnostics| format!("{diagnostics:?}"))
}

fn remove_extension_recursively(root: &Path, extension: &str) {
    for entry in fs::read_dir(root).expect("enumerate artifact root") {
        let path = entry.expect("read artifact entry").path();
        if path.is_dir() {
            remove_extension_recursively(&path, extension);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
            fs::remove_file(path).expect("remove selected artifact extension");
        }
    }
}

fn fresh_cgap_sysroot(dir: &Path) -> String {
    let root = dir.join("fresh_sysroot");
    let external = root.join("libs/external");
    let canonical = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap().join("libs/external");
    copy_tree(&canonical, &external);
    SysrootBuilder::new(Sysroot::from_root(root.clone()).unwrap())
        .build_all(true).expect("build fresh canonical sysroot for artifact parity");
    root.to_string_lossy().to_string()
}

#[test]
fn safe_loan_summaries_exclude_loaded_scalars_and_raw_pointer_only_shapes() {
    let dir = temp_dir();
    let build_root = dir.join("build");
    let build_external = build_root.join("libs/external");
    let canonical_external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap().join("libs/external");
    copy_tree(&canonical_external, &build_external);
    SysrootBuilder::new(Sysroot::from_root(build_root.clone()).unwrap())
        .build_all(true)
        .expect("build fresh canonical artifacts");

    // Force the string provider body through source analysis; all its current
    // dependencies remain freshly-built canonical artifacts.
    let source_root = dir.join("analysis_source");
    let source_external = source_root.join("libs/external");
    copy_tree(&build_external, &source_external);
    for extension in ["llib", "obj"] {
        fs::remove_file(source_external.join("alloc/string").with_extension(extension))
            .expect("remove string artifact to force source body");
    }

    let source = r#"
import <string>;
import <ptr>;
struct OwnedByte { value: u8 };
struct RawHolder { ptr: *u8 };
struct RawPair { ptr: *u8, count: u64 };
struct AnchorOwner { value: i32, ptr: *rw i32 } requires anchor(ptr) = self;
fn load_byte(src: &[u8]) -> u8 { return src[0]; }
fn copy_byte(src: &[u8]) -> OwnedByte {
    dec value = src[0];
    return OwnedByte { value: value };
}
fn raw_passthrough(ptr: *u8) -> *u8 { return ptr; }
unsafe fn raw_identity(ptr: *u8) -> *u8 { return ptr; }
unsafe fn raw_via_helper(ptr: *u8) -> *u8 { return raw_identity(ptr); }
unsafe fn raw_offset(ptr: *rw i32) -> *rw i32 { return (ptr as u64 + 0 as u64) as *rw i32; }
fn raw_field(holder: &RawHolder) -> *u8 { return holder.ptr; }
unsafe fn raw_pair(ptr: *u8) -> RawPair { return RawPair { ptr: ptr, count: 1 as u64 }; }
unsafe fn raw_pair_via_helper(ptr: *u8) -> RawPair { return raw_pair(ptr); }
fn borrow_value(value: &i32) -> &i32 life_from(value) { return value; }
unsafe fn raw_to_safe(pointer: *i32) -> &i32 life_from(pointer) { unsafe { return &*pointer; } }
struct Pair { r: &u8, n: u64 };
fn pair_mixed(r: &u8, data: &[u8]) -> Pair {
    dec n = data[0] as u64;
    return Pair { r: r, n: n };
}
fn owned_string(src: &[u8]) -> std::Option<std::String> { return std::string_from_bytes(src); }
unsafe fn anchor_roundtrip(owner: &rw AnchorOwner) -> &rw i32 life_from(owner) {
    unsafe { owner.ptr = &rw owner.value as *rw i32; }
    dec raw = owner.ptr;
    return &rw *raw;
}
unsafe fn anchor_raw_field(owner: &rw AnchorOwner) -> *rw i32 { return owner.ptr; }
unsafe fn contract_anchor_raw_through_helper(owner: &rw AnchorOwner) -> *rw i32 {
    return anchor_raw_field(owner);
}
unsafe fn anchor_unknown(owner: &rw AnchorOwner) -> &rw i32 life_from(owner) {
    dec raw = owner.ptr;
    return &rw *raw;
}
fn anchor_safe_overwrite(owner: &rw AnchorOwner, other: *rw i32) {
    owner.ptr = other;
}
"#;
    let (module, context) = compile_source_module(&source_root, source, &dir.join("micro_probes.ln"));
    let mut interprocedural = luna_borrowck::interprocedural::InterproceduralContext::new(&context);
    interprocedural.compute_summaries(&module);
    let summaries = &interprocedural.summaries;

    assert_eq!(return_effect(&module, summaries, "load_byte"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "copy_byte"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "raw_passthrough"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "raw_identity"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "raw_via_helper"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "raw_field"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "borrow_value"), ReturnEffect::BorrowsCarried(vec![0]));
    assert_eq!(return_effect(&module, summaries, "raw_to_safe"), ReturnEffect::Independent,
        "unknown raw-pointer promotion must not become an outward safe-borrow effect");
    assert_eq!(return_effect(&module, summaries, "pair_mixed"), ReturnEffect::BorrowsCarried(vec![0]));
    assert_eq!(return_effect(&module, summaries, "owned_string"), ReturnEffect::Independent);
    assert_eq!(return_effect(&module, summaries, "_MFN3std17string_from_bytesE"), ReturnEffect::Independent);

    // Raw pointer origin has its own interprocedural summary domain. It must
    // survive passthrough/field extraction without becoming a safe loan.
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "raw_passthrough"),
        RawPointerReturnEffect::From(vec![0]),
    );
    assert_eq!(raw_pointer_return_effect(&module, summaries, "raw_identity"), RawPointerReturnEffect::From(vec![0]));
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "raw_via_helper"),
        RawPointerReturnEffect::From(vec![0]),
    );
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "raw_offset"),
        RawPointerReturnEffect::From(vec![0]),
        "integer-address arithmetic in a pointer-preserving helper must preserve raw origin",
    );
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "add_mut"),
        RawPointerReturnEffect::From(vec![0]),
        "the canonical generic pointer-offset helper must expose its raw origin summary",
    );
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "raw_field"),
        RawPointerReturnEffect::Unknown,
    );
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "load_byte"),
        RawPointerReturnEffect::Independent,
    );
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "copy_byte"),
        RawPointerReturnEffect::Independent,
    );
    assert_eq!(
        raw_pointer_return_effect(&module, summaries, "owned_string"),
        RawPointerReturnEffect::Independent,
    );
    for name in ["raw_pair", "raw_pair_via_helper"] {
        let function = module.functions.iter().find(|function| function.name.name.contains(name))
            .unwrap_or_else(|| panic!("missing {name}"));
        let fields = &summaries[&function.name].raw_pointer_field_ret;
        assert_eq!(fields.len(), 1, "{name} should summarize only its raw-pointer field");
        assert_eq!(fields["ptr"].origin, RawPointerReturnEffect::From(vec![0]), "{name}");
        assert_eq!(fields["ptr"].anchor, RawPointerAnchorReturnEffect::From(vec![RawPointerAnchorSource::RawParam(0)]), "{name}");
    }
    let expected_owner_field = RawPointerAnchorReturnEffect::From(vec![
        RawPointerAnchorSource::OwnerField { param: 0, field: "ptr".to_string() },
    ]);
    assert_eq!(raw_pointer_anchor_return_effect(&module, summaries, "anchor_raw_field"), expected_owner_field);
    assert_eq!(raw_pointer_anchor_return_effect(&module, summaries, "contract_anchor_raw_through_helper"), expected_owner_field);

    // Representation queries keep raw-pointer capability visible while safe
    // loan shape excludes it. This guards against fixing borrowck by weakening
    // the global pointer/reference query used by other semantic consumers.
    let raw_pointer = module.functions.iter().find(|f| f.name.name.contains("raw_passthrough")).unwrap().ret_ty;
    assert!(context.types.contains_pointer_or_reference(raw_pointer));
    assert!(!context.types.contains_safe_reference(raw_pointer));

    // A real safe reference returned through the provider API must remain a
    // borrow in both source and freshly-built .llib consumer sessions.
    let borrowed_consumer = r#"
import <string>;
fn main() {
    dec rw value = std::string_from_str("x");
    dec bytes = value.as_bytes();
    value.push_char('y');
    dec n = bytes.len;
}
"#;
    let source_consumer_root = dir.join("string_source_consumer");
    let source_consumer_external = source_consumer_root.join("libs/external");
    copy_tree(&build_external, &source_consumer_external);
    for extension in ["llib", "obj"] {
        fs::remove_file(source_consumer_external.join("alloc/string").with_extension(extension))
            .expect("force source string provider");
    }

    let artifact_consumer_root = dir.join("artifact_consumer");
    let artifact_consumer_external = artifact_consumer_root.join("libs/external");
    copy_tree(&build_external, &artifact_consumer_external);
    remove_extension_recursively(&artifact_consumer_external, "ln");

    // These field-content adversaries run against independently prepared
    // fresh source-only and artifact-only sysroots. The Holder type remains a
    // user type in the consumer, so the test isolates field-value provenance
    // from provider-name or stdlib behavior.
    let source_only_root = dir.join("source_only_field_provenance");
    let source_only_external = source_only_root.join("libs/external");
    copy_tree(&build_external, &source_only_external);
    for extension in ["llib", "obj"] {
        remove_extension_recursively(&source_only_external, extension);
    }

    let artifact_only_root = dir.join("artifact_only_field_provenance");
    let artifact_only_external = artifact_only_root.join("libs/external");
    copy_tree(&build_external, &artifact_only_external);
    remove_extension_recursively(&artifact_only_external, "ln");

    let source_result = compile_consumer(&source_consumer_root, borrowed_consumer, &dir.join("consumer_source"));
    let artifact_result = compile_consumer(&artifact_consumer_root, borrowed_consumer, &dir.join("consumer_artifact"));
    for (mode, result) in [("source", &source_result), ("artifact", &artifact_result)] {
        assert!(
            result.as_ref().err().is_some_and(|error| error.contains("BorrowConflict")),
            "live String::as_bytes borrow must reject mutation in {mode} mode: {result:?}"
        );
    }

    // A reference returned by an ordinary user function is a call-site
    // reborrow. Its local loan must remain live through a later use of the
    // result, even though its ultimate source is an external reference
    // parameter. Conversely, NLL must allow mutation after the result is dead.
    let call_reborrow_liveness = r#"
fn borrow_value(value: &rw i32) -> &rw i32 life_from(value) { return value; }
fn mutate_while_live(value: &rw i32) {
    dec borrowed = borrow_value(value);
    *value = 10;
    *borrowed = 11;
}
fn mutate_after_death(value: &rw i32) {
    dec borrowed = borrow_value(value);
    dec observed = *borrowed;
    *value = 12;
}
fn main() {}
"#;
    let live_reborrow_result = compile_consumer(
        &artifact_consumer_root,
        call_reborrow_liveness,
        &dir.join("ordinary_call_reborrow_live"),
    );
    assert!(
        live_reborrow_result.as_ref().err().is_some_and(|error| error.contains("BorrowConflict")),
        "a live function-returned mutable reborrow must conflict with parent mutation: {live_reborrow_result:?}"
    );

    let dead_reborrow_result = compile_consumer(
        &artifact_consumer_root,
        r#"
fn borrow_value(value: &rw i32) -> &rw i32 life_from(value) { return value; }
fn mutate_after_death(value: &rw i32) {
    dec borrowed = borrow_value(value);
    dec observed = *borrowed;
    *value = 12;
}
fn main() {}
"#,
        &dir.join("ordinary_call_reborrow_dead"),
    );
    assert!(
        dead_reborrow_result.is_ok(),
        "NLL must release a returned reborrow after its final use: {dead_reborrow_result:?}"
    );

    // A raw pointer parameter is not a proof of referent origin. Even an
    // explicit lifetime relation cannot turn an arbitrary FFI-style address
    // into a safe returned reference.
    let raw_without_origin = r#"
unsafe fn fabricate(ptr: *rw i32) -> &rw i32 life_from(ptr) {
    return &rw *ptr;
}
fn main() {}
"#;
    let raw_result = compile_consumer(
        &artifact_consumer_root,
        raw_without_origin,
        &dir.join("raw_pointer_without_origin"),
    );
    assert!(
        raw_result.as_ref().err().is_some_and(|error| error.contains("LocalBorrowEscape") || error.contains("lifetime")),
        "raw pointer input must not manufacture safe-reference origin via life_from: {raw_result:?}"
    );

    // The address of Holder.ptr is not the origin of the raw pointer value
    // stored in that slot. With no stored-value fact in this function, the
    // loaded value is unknown and life_from(h) must not manufacture origin.
    let unknown_pointer_field = r#"
import <string>;
struct Holder { ptr: *rw i32 };
unsafe fn expose(h: &rw Holder) -> &rw i32 life_from(h) {
    dec ptr = h.ptr;
    return &rw *ptr;
}
fn main() { dec value = std::string_from_str(""); }
"#;

    // The interprocedural summary path must obey the same value/address
    // distinction. Returning h.ptr as a raw pointer cannot summarize the
    // pointer value as originating from h merely because its slot is in h.
    let unknown_pointer_field_via_helper = r#"
import <string>;
struct Holder { ptr: *rw i32 };
unsafe fn raw_field(h: &rw Holder) -> *rw i32 { return h.ptr; }
unsafe fn expose_via_helper(h: &rw Holder) -> &rw i32 life_from(h) {
    dec ptr = raw_field(h);
    return &rw *ptr;
}
fn main() { dec value = std::string_from_str(""); }
"#;

    // This crosses the raw-return summary channel: the helper receives a raw
    // address of the Holder (known to originate from h), but that does not
    // establish the origin of the unrelated raw pointer value stored in it.
    let unknown_pointer_field_via_raw_helper = r#"
import <string>;
struct Holder { ptr: *rw i32 };
unsafe fn raw_field(h: *rw Holder) -> *rw i32 { return (*h).ptr; }
unsafe fn expose_via_raw_helper(h: &rw Holder) -> &rw i32 life_from(h) {
    dec raw_holder = h as *rw Holder;
    dec ptr = raw_field(raw_holder);
    return &rw *ptr;
}
fn main() { dec value = std::string_from_str(""); }
"#;

    // A known pointer value derived from B is stored in A's field. The field
    // slot's address belongs to A, but the loaded raw pointer still originates
    // from B and cannot be returned under life_from(A).
    let pointer_from_other_owner_field = r#"
import <string>;
struct Holder { ptr: *rw i32 };
unsafe fn expose_foreign(holder_a: &rw Holder, owner_b: &rw i32) -> &rw i32 life_from(holder_a) {
    holder_a.ptr = owner_b as *rw i32;
    dec ptr = holder_a.ptr;
    return &rw *ptr;
}
fn main() { dec value = std::string_from_str(""); }
"#;

    for (mode, root) in [
        ("source", &source_only_root),
        ("fresh artifact-only", &artifact_only_root),
    ] {
        for (case, source) in [
            ("unknown pointer field", unknown_pointer_field),
            ("unknown pointer field via helper", unknown_pointer_field_via_helper),
            ("unknown pointer field via raw helper", unknown_pointer_field_via_raw_helper),
            ("pointer from B stored in A", pointer_from_other_owner_field),
        ] {
            let result = compile_consumer(
                root,
                source,
                &dir.join(format!("field_origin_{mode}_{case}")),
            );
            assert!(
                result.as_ref().err().is_some_and(|error| {
                    error.contains("LocalBorrowEscape")
                        || error.contains("LifetimeConstraintViolation")
                        || error.contains("lifetime")
                }),
                "{case} must not be blessed by Holder's field-address origin in {mode} mode: {result:?}"
            );
        }
    }

    // A real raw origin is necessary, but an unrelated lifetime contract is
    // not enough to bless it. The origin is `owner_a`; claiming `owner_b`
    // must be rejected by the normal provenance/region contract path.
    let mismatched_owner = r#"
unsafe fn borrow_a_as_b(owner_a: &rw i32, owner_b: &rw i32) -> &rw i32 life_from(owner_b) {
    dec ptr = owner_a as *rw i32;
    return &rw *ptr;
}
fn main() {}
"#;
    let mismatch_result = compile_consumer(
        &artifact_consumer_root,
        mismatched_owner,
        &dir.join("raw_origin_owner_mismatch"),
    );
    assert!(
        mismatch_result.as_ref().err().is_some_and(|error| {
            error.contains("LifetimeConstraintViolation")
                || error.contains("LocalBorrowEscape")
                || error.contains("lifetime")
        }),
        "raw origin from owner_a must not be returned under owner_b's lifetime: {mismatch_result:?}"
    );

    // Joining a known owner with an unknown raw pointer must not erase the
    // unknown alternative and thereby certify every runtime path as owner_a.
    let known_unknown_merge = r#"
unsafe fn borrow_merged(owner_a: &rw i32, foreign: *rw i32, choose_foreign: bool) -> &rw i32 life_from(owner_a) {
    dec rw ptr = owner_a as *rw i32;
    if choose_foreign { ptr = foreign; }
    return &rw *ptr;
}
fn main() {}
"#;
    let merge_result = compile_consumer(
        &artifact_consumer_root,
        known_unknown_merge,
        &dir.join("raw_origin_known_unknown_merge"),
    );
    assert!(
        merge_result.as_ref().err().is_some_and(|error| {
            error.contains("LifetimeConstraintViolation")
                || error.contains("LocalBorrowEscape")
                || error.contains("lifetime")
        }),
        "joining known owner_a with unknown raw origin must not validate as owner_a: {merge_result:?}"
    );

    let known_known_merge = r#"
unsafe fn borrow_merged(owner_a: &rw i32, owner_b: &rw i32, choose_b: bool) -> &rw i32 life_from(owner_a) {
    dec rw ptr = owner_a as *rw i32;
    if choose_b { ptr = owner_b as *rw i32; }
    return &rw *ptr;
}
fn main() {}
"#;
    let known_merge_result = compile_consumer(
        &artifact_consumer_root,
        known_known_merge,
        &dir.join("raw_origin_two_known_merge"),
    );
    assert!(
        known_merge_result.as_ref().err().is_some_and(|error| {
            error.contains("LifetimeConstraintViolation")
                || error.contains("LocalBorrowEscape")
                || error.contains("lifetime")
        }),
        "joining origins from owner_a and owner_b must not satisfy only life_from(owner_a): {known_merge_result:?}"
    );

    // With both pieces of evidence aligned, raw -> safe conversion is valid:
    // the pointer originates at owner_a and the declared relation is also
    // life_from(owner_a).
    let matching_owner = r#"
unsafe fn raw_identity(ptr: *rw i32) -> *rw i32 { return ptr; }
unsafe fn borrow_a(owner_a: &rw i32) -> &rw i32 life_from(owner_a) {
    dec first = owner_a as *rw i32;
    dec second = first;
    dec ptr = raw_identity(second);
    return &rw *ptr;
}
fn main() {}
"#;
    let matching_result = compile_consumer(
        &artifact_consumer_root,
        matching_owner,
        &dir.join("raw_origin_owner_match"),
    );
    assert!(
        matching_result.is_ok(),
        "a known raw origin plus matching life_from relation should be valid: {matching_result:?}"
    );

    // A declared owner contract is independent of raw address origin. At a
    // provider/function boundary the loaded raw origin may be Unknown; the
    // type invariant plus the explicit `life_from(owner)` contract is the
    // evidence for a safe loan. This must behave identically in source and
    // fresh artifact-only sysroots.
    let anchored_contract = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn raw_field(owner: &rw AnchoredOwner) -> *rw i32 { return owner.ptr; }
unsafe fn expose(owner: &rw AnchoredOwner) -> &rw i32 life_from(owner) {
    dec ptr = raw_field(owner);
    dec copied = ptr;
    return &rw *copied;
}
fn main() {
    dec rw target = 0;
    unsafe {
        dec rw owner = AnchoredOwner { ptr: &rw target as *rw i32 };
        dec borrowed = expose(&rw owner);
    }
}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let result = compile_consumer(root, anchored_contract, &dir.join(format!("anchored_contract_{mode}")));
        assert!(result.is_ok(), "declared anchor plus life_from should permit helper-mediated promotion in {mode}: {result:?}");
    }

    // A safe store may preserve an already-proven anchor for the same owner.
    // This function is intentionally safe, so its assignment must lower to an
    // ordinary Store rather than an unsafe establishment operation.
    let safe_same_owner_store = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
fn keep_same_pointer(owner: &rw AnchoredOwner) {
    dec saved = owner.ptr;
    owner.ptr = saved;
}
unsafe fn expose(owner: &rw AnchoredOwner) -> &rw i32 life_from(owner) {
    return &rw *owner.ptr;
}
fn main() {
    dec rw target = 12;
    unsafe {
        dec rw owner = AnchoredOwner { ptr: &rw target as *rw i32 };
        keep_same_pointer(&rw owner);
        dec borrowed = expose(&rw owner);
        *borrowed = 13;
    }
}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let result = compile_consumer(root, safe_same_owner_store, &dir.join(format!("anchored_safe_same_owner_{mode}")));
        assert!(result.is_ok(), "safe same-owner store must preserve the declared anchor in {mode}: {result:?}");
    }

    // Pointer arithmetic through an ordinary stdlib helper preserves the
    // logical anchor while the raw pointer remains outside the safe-loan
    // domain. A zero offset keeps this fixture's referent valid at runtime.
    let anchored_offset = r#"
import <ptr>;
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn expose(owner: &rw AnchoredOwner) -> &rw i32 life_from(owner) {
    dec shifted = std::ptr::add_mut<i32>(owner.ptr, 0 as u64);
    return &rw *shifted;
}
fn main() {
    dec rw target = 17;
    unsafe {
        dec rw owner = AnchoredOwner { ptr: &rw target as *rw i32 };
        dec borrowed = expose(&rw owner);
        *borrowed = 18;
    }
}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let result = compile_consumer(root, anchored_offset, &dir.join(format!("anchored_offset_{mode}")));
        assert!(result.is_ok(), "raw pointer offset must preserve its anchor in {mode}: {result:?}");
    }

    let shared_anchor = r#"
struct SharedOwner { ptr: *i32 } requires anchor(ptr) = self;
unsafe fn expose(owner: &SharedOwner) -> &i32 life_from(owner) {
    return &*owner.ptr;
}
fn main() {
    dec target = 19;
    unsafe {
        dec owner = SharedOwner { ptr: &target as *i32 };
        dec borrowed = expose(&owner);
        if *borrowed != 19 { return; }
    }
}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let result = compile_consumer(root, shared_anchor, &dir.join(format!("anchored_shared_{mode}")));
        assert!(result.is_ok(), "shared raw field should allow a shared anchored reference in {mode}: {result:?}");
    }

    // Moving an owner transfers/rebases its field contract. The pointer value
    // inside the moved aggregate remains the same; the owner-relative proof is
    // reconstructed against the destination value in the new function call.
    let anchored_move = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn expose(owner: &rw AnchoredOwner) -> &rw i32 life_from(owner) {
    dec raw = owner.ptr;
    return &rw *raw;
}
fn main() {
    dec rw target = 7;
    unsafe {
        dec rw first = AnchoredOwner { ptr: &rw target as *rw i32 };
        dec rw moved = first;
        dec borrowed = expose(&rw moved);
        if *borrowed != 7 { return; }
    }
}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let result = compile_consumer(root, anchored_move, &dir.join(format!("anchored_move_{mode}")));
        assert!(result.is_ok(), "moving an anchored owner should rebase its internal field proof in {mode} mode: {result:?}");
    }

    // A copied-out raw pointer must not retain proof for the replaced field
    // value. Re-establishing the field invariant under unsafe does not revive
    // an older pointer value that may have been invalidated by replacement.
    let stale_pointer_after_replacement = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn expose_stale(owner: &rw AnchoredOwner, replacement: *rw i32) -> &rw i32 life_from(owner) {
    dec old = owner.ptr;
    unsafe { owner.ptr = replacement; }
    return &rw *old;
}
fn main() {}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let result = compile_consumer(root, stale_pointer_after_replacement, &dir.join(format!("anchored_stale_{mode}")));
        assert!(result.as_ref().err().is_some_and(|error| error.contains("RawStorageAnchorViolation")),
            "a raw pointer extracted before field replacement must lose its anchor in {mode} mode: {result:?}");
    }

    // Unsafe establishment does not bypass an already-live safe loan of the
    // logical owner. Keep the derived reference live after the store.
    let mutation_while_borrowed = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn expose(owner: &rw AnchoredOwner) -> &rw i32 life_from(owner) {
    dec raw = owner.ptr;
    return &rw *raw;
}
fn replace(owner: &rw AnchoredOwner, replacement: *rw i32) {
    unsafe {
        dec borrowed = expose(owner);
        owner.ptr = replacement;
        *borrowed = 9;
    }
}
fn main() {}
"#;
    let live_loan_result = compile_consumer(
        &artifact_consumer_root,
        mutation_while_borrowed,
        &dir.join("anchored_mutation_while_borrowed"),
    );
    assert!(live_loan_result.as_ref().err().is_some_and(|error| error.contains("BorrowConflict")),
        "unsafe field re-establishment must not bypass a live safe loan: {live_loan_result:?}");

    // A shared owner receiver cannot promote a mutable raw pointer into an
    // exclusive safe reference, even though the field has an anchor contract.
    let shared_owner_mutable = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn expose(owner: &AnchoredOwner) -> &rw i32 life_from(owner) {
    dec ptr = owner.ptr;
    return &rw *ptr;
}
fn main() {}
"#;
    let shared_owner_result = compile_consumer(
        &artifact_consumer_root,
        shared_owner_mutable,
        &dir.join("anchored_shared_owner_mutable"),
    );
    assert!(
        shared_owner_result.as_ref().err().is_some_and(|error| error.contains("RawStorageAnchorViolation")),
        "a shared logical owner must not grant a mutable anchored reference: {shared_owner_result:?}"
    );

    // Safe stores cannot establish an anchor from an unknown/foreign value.
    let unsafe_store_required = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
fn overwrite(owner: &rw AnchoredOwner, foreign: *rw i32) { owner.ptr = foreign; }
fn main() {}
"#;
    let overwrite_result = compile_consumer(
        &artifact_consumer_root,
        unsafe_store_required,
        &dir.join("anchored_safe_overwrite"),
    );
    assert!(
        overwrite_result.as_ref().err().is_some_and(|error| error.contains("RawStorageAnchorMismatch")),
        "safe overwrite with an unproven raw pointer must reject: {overwrite_result:?}"
    );

    // The safe store must be valid on every CFG path. Neither an unknown raw
    // pointer nor a pointer anchored to another owner can be promoted merely
    // because one branch kept this owner's original pointer.
    let unsafe_cfg_store = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
fn overwrite(owner: &rw AnchoredOwner, foreign: *rw i32, choose: bool) {
    dec rw candidate = owner.ptr;
    if choose { candidate = foreign; }
    owner.ptr = candidate;
}
fn main() {}
"#;
    let other_owner_cfg_store = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
fn overwrite(owner_a: &rw AnchoredOwner, owner_b: &rw AnchoredOwner, choose: bool) {
    dec rw candidate = owner_a.ptr;
    if choose { candidate = owner_b.ptr; }
    owner_a.ptr = candidate;
}
fn main() {}
"#;
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        for (case, source) in [("unknown", unsafe_cfg_store), ("other_owner", other_owner_cfg_store)] {
            let result = compile_consumer(root, source, &dir.join(format!("anchored_cfg_store_{mode}_{case}")));
            assert!(
                result.as_ref().err().is_some_and(|error| error.contains("RawStorageAnchorMismatch")),
                "safe store after {case} branch must reject in {mode}: {result:?}"
            );
        }
    }

    // Null is permitted as an empty-owner sentinel, but it is not a usable
    // referent even though its field is covered by the type-level anchor.
    let null_not_promotable = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
unsafe fn expose(owner: &rw AnchoredOwner) -> &rw i32 life_from(owner) {
    dec ptr = owner.ptr;
    return &rw *ptr;
}
fn main() {
    dec rw owner = AnchoredOwner { ptr: 0 as u64 as *rw i32 };
    unsafe {
        dec ptr = owner.ptr;
        dec borrowed = &rw *ptr;
    }
}
"#;
    let null_result = compile_consumer(
        &artifact_consumer_root,
        null_not_promotable,
        &dir.join("anchored_null_not_promotable"),
    );
    assert!(
        null_result.as_ref().err().is_some_and(|error| error.contains("RawStorageAnchorViolation")),
        "the null sentinel must not become a safe reference through an anchor contract: {null_result:?}"
    );

    // A direct immutable local does not expose a mutable logical owner just
    // because its field stores a `*rw` pointer. Using `&rw owner` above is the
    // positive path: the language has already checked that local binding's
    // mutability before the anchored receiver contract is applied.
    let immutable_local_owner = r#"
struct AnchoredOwner { ptr: *rw i32 } requires anchor(ptr) = self;
fn main() {
    dec rw target = 0;
    unsafe {
        dec owner = AnchoredOwner { ptr: &rw target as *rw i32 };
        dec ptr = owner.ptr;
        dec borrowed = &rw *ptr;
    }
}
"#;
    let immutable_local_result = compile_consumer(
        &artifact_consumer_root,
        immutable_local_owner,
        &dir.join("anchored_immutable_local_owner"),
    );
    assert!(
        immutable_local_result.as_ref().err().is_some_and(|error| error.contains("RawStorageAnchorViolation")),
        "a direct immutable local owner must not grant a mutable anchored borrow: {immutable_local_result:?}"
    );

    // RawTable's zero-capacity null sentinels must remain non-promotable, but
    // get_or_insert must grow/establish storage and work identically when the
    // provider graph is loaded from source or from fresh canonical artifacts.
    let raw_table_growth = r#"
import <hashmap>;
fn main() -> i32 {
    dec rw map = std::hashmap_new<i32, i32>();
    dec rw inserted = map.get_or_insert(10, 500);
    if *inserted != 500 { return 1; }
    *inserted = 555;
    dec rw existing = map.get_or_insert(10, 999);
    if *existing != 555 { return 2; }
    return 0;
}
"#;
    let mut raw_table_results = Vec::new();
    for (mode, root) in [("source", &source_only_root), ("fresh artifact-only", &artifact_only_root)] {
        let binary_dir = dir.join(format!("rawtable_get_or_insert_{mode}"));
        let result = compile_consumer(root, raw_table_growth, &binary_dir);
        assert!(result.is_ok(), "RawTable::get_or_insert should compile in {mode} mode: {result:?}");
        let binary = binary_dir.join(if cfg!(windows) { "main.exe" } else { "main" });
        let status = Command::new(&binary).status().expect("run RawTable fixture");
        raw_table_results.push((mode, status.code()));
    }
    assert_eq!(raw_table_results, [("source", Some(0)), ("fresh artifact-only", Some(0))]);
}

#[test]
fn raw_storage_anchor_type_contract_survives_provider_llib_boundary() {
    let dir = temp_dir();
    let sysroot = fresh_cgap_sysroot(&dir);
    let provider = r#"
module anchor_api {
    export struct Owner {
        export ptr: *rw i32,
    } requires anchor(ptr) = self;
}
"#;

    let source_dir = dir.join("source_provider");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("anchor_api.ln"), provider).unwrap();

    let artifact_dir = dir.join("artifact_provider");
    fs::create_dir_all(&artifact_dir).unwrap();
    let provider_path = artifact_dir.join("anchor_api.ln");
    let artifact_path = artifact_dir.join("anchor_api.llib");
    fs::write(&provider_path, provider).unwrap();
    let provider_options = CompilerOptions {
        output_path: Some(artifact_path.to_string_lossy().to_string()),
        search_paths: vec![artifact_dir.to_string_lossy().to_string(), sysroot.clone()],
        emit_llib: true,
        no_link: true,
        quiet: true,
        ..Default::default()
    };
    compile(provider_path.to_str().unwrap(), provider.to_string(), &provider_options)
        .expect("freshly build user-defined anchored provider .llib");
    let provider_obj = artifact_dir.join("anchor_api.obj");
    assert!(artifact_path.exists(), "fresh .llib must exist");
    assert!(provider_obj.exists(), "fresh provider .obj must exist");
    fs::remove_file(&provider_path).expect("force artifact-only provider resolution");

    let positive = r#"
import "anchor_api";
unsafe fn expose(owner: &rw anchor_api::Owner) -> &rw i32 life_from(owner) {
    dec raw = owner.ptr;
    return &rw *raw;
}
fn main() -> i32 {
    dec rw target = 41;
    unsafe {
        dec rw owner = anchor_api::Owner { ptr: &rw target as *rw i32 };
        dec borrowed = expose(&rw owner);
        if *borrowed == 41 { return 0; }
    }
    return 1;
}
"#;
    let compile_and_run = |root: &Path, mode: &str| -> (i32, String) {
        let source_path = root.join(format!("consumer_{mode}.ln"));
        let output_path = root.join(if cfg!(windows) { format!("consumer_{mode}.exe") } else { format!("consumer_{mode}") });
        fs::write(&source_path, positive).unwrap();
        let options = CompilerOptions {
            output_path: Some(output_path.to_string_lossy().to_string()),
            search_paths: vec![root.to_string_lossy().to_string(), sysroot.clone()],
            quiet: true,
            ..Default::default()
        };
        compile(source_path.to_str().unwrap(), positive.to_string(), &options)
            .unwrap_or_else(|errors| panic!("{mode} consumer must compile using imported anchor contract: {errors:?}"));
        let output = Command::new(&output_path).output().expect("run anchored consumer");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).to_string(),
        )
    };

    let source_result = compile_and_run(&source_dir, "source");
    let artifact_result = compile_and_run(&artifact_dir, "artifact_only");
    assert_eq!(source_result, (0, String::new()));
    assert_eq!(artifact_result, source_result, "source and fresh .llib type contracts must be observationally equivalent");

    let negative = r#"
import "anchor_api";
unsafe fn claim_a_as_b(a: &rw anchor_api::Owner, b: &rw anchor_api::Owner) -> &rw i32 life_from(b) {
    dec raw = a.ptr;
    return &rw *raw;
}
fn main() {}
"#;
    for (mode, root) in [("source", &source_dir), ("artifact-only", &artifact_dir)] {
        let source_path = root.join(format!("negative_{mode}.ln"));
        fs::write(&source_path, negative).unwrap();
        let options = CompilerOptions {
            search_paths: vec![root.to_string_lossy().to_string(), sysroot.clone()],
            quiet: true,
            ..Default::default()
        };
        let result = compile(source_path.to_str().unwrap(), negative.to_string(), &options);
        let errors = result.expect_err("contract anchored to A must not be re-labeled as B");
        let diagnostics = format!("{errors:?}");
        assert!(
            diagnostics.contains("RawStorageAnchorViolation")
                || diagnostics.contains("LocalBorrowEscape")
                || diagnostics.contains("LifetimeConstraintViolation"),
            "wrong-owner raw-to-safe conversion must reject in {mode}: {diagnostics}"
        );
    }
}

#[test]
fn aggregate_raw_field_origin_survives_call_and_fresh_provider_artifact() {
    let dir = temp_dir();
    let sysroot = fresh_cgap_sysroot(&dir);
    let provider = r#"
module raw_pair_api {
    export struct Pair { export ptr: *rw i32, export count: u64 };
    export fn pack(ptr: *rw i32) -> Pair {
        return Pair { ptr: ptr, count: 1 as u64 };
    }
    export fn relay(ptr: *rw i32) -> Pair { return pack(ptr); }
}
"#;
    let source_dir = dir.join("source");
    fs::create_dir_all(&source_dir).unwrap();
    fs::write(source_dir.join("raw_pair_api.ln"), provider).unwrap();

    let artifact_dir = dir.join("artifact");
    fs::create_dir_all(&artifact_dir).unwrap();
    let provider_path = artifact_dir.join("raw_pair_api.ln");
    fs::write(&provider_path, provider).unwrap();
    let artifact_path = artifact_dir.join("raw_pair_api.llib");
    let options = CompilerOptions {
        output_path: Some(artifact_path.to_string_lossy().to_string()),
        search_paths: vec![artifact_dir.to_string_lossy().to_string(), sysroot.clone()],
        emit_llib: true, no_link: true, quiet: true,
        ..Default::default()
    };
    compile(provider_path.to_str().unwrap(), provider.to_string(), &options)
        .expect("build fresh aggregate raw-field provider artifact");
    assert!(artifact_path.exists());
    assert!(artifact_dir.join("raw_pair_api.obj").exists());
    let (_, _, _, semantic_metadata) = luna_llib::reader::MlibReader::read_module(
        &mut fs::File::open(&artifact_path).expect("open fresh provider artifact"),
    ).expect("read fresh provider semantic metadata");
    let semantic_metadata = semantic_metadata.expect("provider artifact must carry canonical semantic metadata");
    fn find_symbol<'a>(
        symbols: &'a std::collections::BTreeMap<String, luna_llib::metadata::ExportedSymbol>,
        path: &str,
    ) -> Option<&'a luna_llib::metadata::ExportedSymbol> {
        for symbol in symbols.values() {
            if symbol.symbol_id.symbol_path == path { return Some(symbol); }
            if let Some(found) = find_symbol(&symbol.children, path) { return Some(found); }
        }
        None
    }
    let relay = find_symbol(&semantic_metadata.interface.exported_symbols, "raw_pair_api::relay")
        .expect("stable exported function identity must be present in semantic metadata");
    assert_eq!(
        relay.raw_pointer_effects.as_ref().and_then(|effects| effects.direct_fields.get("ptr"))
            .map(|effect| &effect.origin),
        Some(&luna_llib::metadata::CanonicalRawPointerOrigin::FromParameters(vec![0])),
        "aggregate field raw origin must use a stable parameter-relative summary",
    );
    fs::remove_file(provider_path).expect("force artifact-only resolution");

    let consumer = r#"
import "raw_pair_api";
unsafe fn expose(owner: &rw i32) -> &rw i32 life_from(owner) {
    dec pair = raw_pair_api::relay(owner as *rw i32);
    return &rw *pair.ptr;
}
fn main() -> i32 {
    dec rw value = 41;
    unsafe {
        dec borrowed = expose(&rw value);
        if *borrowed == 41 { return 0; }
    }
    return 1;
}
"#;
    let mut observations = Vec::new();
    for (mode, root) in [("source", &source_dir), ("artifact", &artifact_dir)] {
        let source_path = root.join(format!("consumer_{mode}.ln"));
        let binary_path = root.join(if cfg!(windows) { format!("consumer_{mode}.exe") } else { format!("consumer_{mode}") });
        fs::write(&source_path, consumer).unwrap();
        let options = CompilerOptions {
            output_path: Some(binary_path.to_string_lossy().to_string()),
            search_paths: vec![root.to_string_lossy().to_string(), sysroot.clone()],
            quiet: true, ..Default::default()
        };
        compile(source_path.to_str().unwrap(), consumer.to_string(), &options)
            .unwrap_or_else(|errors| panic!("{mode} aggregate raw-field consumer failed: {errors:?}"));
        let result = Command::new(binary_path).output().expect("run aggregate raw-field consumer");
        observations.push((result.status.code(), result.stdout));
    }
    assert_eq!(observations[0], (Some(0), Vec::new()));
    assert_eq!(observations[1], observations[0], "source and fresh artifact must agree");
}

#[test]
fn unknown_raw_pointer_loans_are_local_and_never_escape() {
    let dir = temp_dir();
    let sysroot = PathBuf::from(fresh_cgap_sysroot(&dir));
    let source_root = dir.join("source_mode");
    let artifact_root = dir.join("artifact_mode");
    copy_tree(&sysroot, &source_root);
    copy_tree(&sysroot, &artifact_root);
    remove_extension_recursively(&artifact_root.join("libs/external"), "ln");

    let positives = [
        ("transient", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_transient.ln")),
        ("trait_dispatch", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_trait_dispatch.ln")),
        ("shared_shared", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_shared_shared.ln")),
        ("dead_loan", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_dead_loan.ln")),
    ];
    for (case, source) in positives {
        let mut observations = Vec::new();
        for (mode, root) in [("source", &source_root), ("artifact", &artifact_root)] {
            let case_dir = dir.join(format!("{case}_{mode}"));
            compile_consumer(root, source, &case_dir)
                .unwrap_or_else(|errors| panic!("{case} must compile in {mode} mode: {errors}"));
            let output_path = case_dir.join(if cfg!(windows) { "main.exe" } else { "main" });
            let result = Command::new(output_path).output().expect("run unsafe raw-root regression");
            observations.push((result.status.code(), result.stdout, result.stderr));
        }
        assert_eq!(observations[0], observations[1], "source/.llib parity for {case}");
        assert_eq!(observations[0].0, Some(0), "{case} executable must succeed");
    }

    let negatives = [
        ("return_reference", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_return_reference.ln")),
        ("trait_return_escape", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_trait_return_escape.ln")),
        ("return_aggregate", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_return_aggregate.ln")),
        ("unrelated_lifetime", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_unrelated_lifetime.ln")),
        ("mutable_shared_conflict", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_mutable_shared_conflict.ln")),
        ("mutable_mutable_conflict", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_mutable_mutable_conflict.ln")),
        ("copy_offset_conflict", include_str!("../../../tests/luna/language/raw_pointer/unsafe_raw_root_copy_offset_conflict.ln")),
    ];
    for (case, source) in negatives {
        for (mode, root) in [("source", &source_root), ("artifact", &artifact_root)] {
            let case_dir = dir.join(format!("{case}_{mode}"));
            let result = compile_consumer(root, source, &case_dir);
            let errors = result.expect_err(&format!("{case} must reject in {mode} mode"));
            assert!(
                errors.contains("LocalBorrowEscape") || errors.contains("BorrowConflict"),
                "{case} rejected for the wrong reason in {mode}: {errors}"
            );
        }
    }
}
