// Diagnostic-only: mirrors the driver's pre-MVIR phase order without changing it.
// This is not a replacement for CLI acceptance testing.
use std::{env, fs, io::{self, Write}, time::Instant};
use luna_ast::AstArena;
use luna_common::CompilerSession;
use luna_lexer::Lexer;
use luna_parser::Parser;
use luna_semantic::{SemanticContext, Resolver, TypeChecker};

fn mark(stage: &str, started: Instant) {
    println!("PHASE {stage} elapsed={:.6}s", started.elapsed().as_secs_f64());
    io::stdout().flush().unwrap();
}

fn metadata(registry: &luna_driver::registry::ModuleRegistry) {
    let mut interfaces: Vec<_> = registry.interfaces.values().collect();
    interfaces.sort_by(|a, b| a.name.cmp(&b.name));
    for interface in interfaces {
        let bounds: Vec<_> = interface.trait_bounds.values().flatten().collect();
        let unique: std::collections::HashSet<_> = bounds.iter()
            .map(|b| (&b.param, &b.trait_id, &b.trait_args)).collect();
        let local = bounds.iter().filter(|b| b.param.provider_id == interface.id).count();
        println!("METADATA provider={} bounds={} unique={} local={} foreign={} assoc={} symbols={}",
            interface.name, bounds.len(), unique.len(), local, bounds.len() - local,
            interface.assoc_type_bounds.values().map(Vec::len).sum::<usize>(),
            interface.symbol_canonicals.len());
    }
    io::stdout().flush().unwrap();
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let file_name = &args[1];
    let root = &args[2];
    let internal = args.get(3).is_some_and(|s| s.starts_with("internal"));
    let bootstrap_internal = args.get(3).is_some_and(|s| s == "internal-bootstrap");
    let started = Instant::now();
    let input = fs::read_to_string(file_name).unwrap();
    let mut session = CompilerSession::new();
    let file_id = session.source_manager.add_file(file_name.clone(), input.clone());
    let mut arena = AstArena::new();
    let mut parser = Parser::new(Lexer::new(&input, file_id), &mut arena, file_id);
    let mut items = parser.parse_file().expect("parse");
    assert!(parser.diagnostics.is_empty(), "parse diagnostics");
    let mut ctx = SemanticContext::new();
    mark("parsed", started);
    let paths = vec![root.clone()];
    let sysroot = luna_driver::sysroot::Sysroot::from_root(root.into()).unwrap();
    let mut driver = luna_driver::DriverSession::new(sysroot, &mut session, &paths);
    if internal && !bootstrap_internal {
        ctx.allow_internal_lang_items = true;
    } else {
        driver.bootstrap_lang_contracts(&mut arena).unwrap();
    }
    mark("bootstrap", started);
    metadata(&driver.registry);
    let context = if internal {
        luna_driver::resolution_context::ProviderResolutionContext::SysrootDependency
    } else {
        luna_driver::resolution_context::ProviderResolutionContext::UserImport
    };
    let imports = luna_driver::importer::get_imports(&items, &arena, &driver);
    for import in imports {
        mark(&format!("load_{}_begin", import.0), started);
        luna_driver::importer::resolve_collected_imports(vec![import], &mut arena, &mut driver, context).unwrap();
        mark("load_done", started);
        metadata(&driver.registry);
    }
    let registry = std::mem::take(&mut driver.registry);
    drop(driver);
    mark("imports", started);
    let mut attrs = luna_semantic::AttributeProcessor::new(&mut arena, &mut session.source_manager, file_id);
    let items = attrs.process_items(items).unwrap();
    mark("attributes", started);
    registry.inject_into_ctx(&mut ctx);
    mark("registry", started);
    Resolver::new(&mut ctx, &arena, &session.source_manager).register_macros(&items);
    assert!(ctx.diagnostics.is_empty(), "macro registration diagnostics");
    let mut macros = luna_semantic::MacroEngine::new(&mut arena, &session.source_manager,
        file_id, &ctx.symbol_table, &ctx.tables);
    let items = macros.expand_items(items).unwrap();
    mark("macros", started);
    Resolver::new(&mut ctx, &arena, &session.source_manager).resolve_items(&items);
    mark("resolved", started);
    let engine = luna_mvir::MvirComptimeEngine { max_steps: 1_000_000, max_depth: 512 };
    TypeChecker::new_with_engine(&mut ctx, &arena, &session.source_manager, &engine).typecheck_items(&items);
    mark("typechecked", started);
    println!("DIAGNOSTICS before_mono={}", ctx.diagnostics.len());
    let mut mono = luna_semantic::MonoCollector::new_with_source(&mut ctx, &arena, Some(&session.source_manager));
    mono.run(&items);
    println!("MONO instances={}", mono.instantiated.len());
    mark("monomorphized", started);
    for diagnostic in &ctx.diagnostics {
        println!("{}", diagnostic.render(&session.source_manager));
    }
}
