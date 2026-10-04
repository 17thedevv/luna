pub mod importer;
pub mod registry;
pub mod async_lowering;
pub mod sysroot;
pub mod external;
pub mod sysroot_builder;
pub mod error;
pub mod session;
pub mod discovery;
pub mod metadata_builder;
pub mod metadata_decoder;
pub mod lang_contracts;
pub mod sysroot_manifest;
pub mod resolution_context;
pub mod provider_binding;

pub use session::DriverSession;

use luna_ast::AstArena;
use luna_common::{CompilerSession, Diagnostic};
use luna_lexer::Lexer;
use luna_parser::Parser;
use luna_semantic::{SemanticContext, Resolver, TypeChecker};
use luna_mvir::{MvirGenerator, print_module};
use luna_backend::{LLVMBackend, TargetConfig, link_objs_to_exe};

fn seed_imported_raw_pointer_effects(
    interproc: &mut luna_borrowck::interprocedural::InterproceduralContext<'_>,
    module: &luna_mvir::Module,
    registry: &registry::ModuleRegistry,
    semantic_ctx: &SemanticContext,
) {
    use luna_borrowck::effect::{
        CallEffectSummary, RawPointerAnchorReturnEffect, RawPointerAnchorSource,
        RawPointerFieldReturnEffect, RawPointerReturnEffect,
    };
    use luna_llib::metadata::{
        CanonicalRawPointerAnchor, CanonicalRawPointerAnchorSource,
        CanonicalRawPointerOrigin,
    };

    let decode_effect = |effect: &luna_llib::metadata::CanonicalRawPointerEffect| {
        let origin = match &effect.origin {
            CanonicalRawPointerOrigin::Independent => RawPointerReturnEffect::Independent,
            CanonicalRawPointerOrigin::FromParameters(params) => {
                RawPointerReturnEffect::From(params.iter().map(|index| *index as usize).collect())
            }
            CanonicalRawPointerOrigin::Unknown => RawPointerReturnEffect::Unknown,
        };
        let anchor = match &effect.anchor {
            CanonicalRawPointerAnchor::Independent => RawPointerAnchorReturnEffect::Independent,
            CanonicalRawPointerAnchor::Unknown => RawPointerAnchorReturnEffect::Unknown,
            CanonicalRawPointerAnchor::From(sources) => RawPointerAnchorReturnEffect::From(
                sources.iter().map(|source| match source {
                    CanonicalRawPointerAnchorSource::RawParameter(index) => {
                        RawPointerAnchorSource::RawParam(*index as usize)
                    }
                    CanonicalRawPointerAnchorSource::OwnerField { parameter, field_name } => {
                        RawPointerAnchorSource::OwnerField { param: *parameter as usize, field: field_name.clone() }
                    }
                    CanonicalRawPointerAnchorSource::Unknown => RawPointerAnchorSource::Unknown,
                }).collect(),
            ),
        };
        (origin, anchor)
    };

    for function in &module.functions {
        for block in &function.blocks {
            for &value_id in &block.insts {
                let Some(value) = function.values.get(value_id.0 as usize) else { continue; };
                let luna_mvir::Instruction::CallDirect { callee, args } = &value.inst else { continue; };
                let Some(symbol_id) = callee.symbol_id else { continue; };
                let symbol = semantic_ctx.symbol_table.get_symbol(symbol_id);
                let Some(provider_id) = symbol.provider_id else { continue; };
                let Some(provider) = registry.interfaces.get(&provider_id) else { continue; };
                let path = semantic_ctx.symbol_table.get_full_logical_path(symbol_id).join("::");
                let Some(canonical) = provider.raw_pointer_effects_by_path.get(&path) else { continue; };

                // Preserve conservative non-raw call behavior: this artifact
                // supplement only refines raw-pointer origin/anchor channels.
                let mut summary = CallEffectSummary::worst_case(args.len());
                let (origin, anchor) = decode_effect(&canonical.returned);
                summary.raw_pointer_ret = origin;
                summary.raw_pointer_anchor_ret = anchor;
                for (field_name, effect) in &canonical.direct_fields {
                    let (origin, anchor) = decode_effect(effect);
                    summary.raw_pointer_field_ret.insert(
                        field_name.clone(),
                        RawPointerFieldReturnEffect { origin, anchor },
                    );
                }
                interproc.seed_summary(callee.clone(), summary);
            }
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct CompilerOptions {
    pub output_path: Option<String>,
    pub emit_llvm: bool,
    pub emit_mvir: bool,
    pub emit_llib: bool,
    pub emit_mlib: bool,
    pub search_paths: Vec<String>,
    pub quiet: bool,
    pub no_link: bool,
    pub comptime_steps: Option<usize>,
    pub comptime_depth: Option<usize>,
    pub is_sysroot_build: bool,
    pub provider_bindings: provider_binding::ProviderBindings,
}

fn verify_items_lifetime(
    items: &[luna_ast::Item],
    arena: &AstArena,
    semantic_ctx: &SemanticContext,
    source_manager: &luna_common::SourceManager,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        if let luna_ast::Item::Decl(decl_id) = item {
            let decl = &arena.decls[decl_id.0 as usize];
            match decl {
                luna_ast::Decl::Module { items: inner_decls, .. } => {
                    let inner_items: Vec<luna_ast::Item> = inner_decls.iter().map(|&d| luna_ast::Item::Decl(d)).collect();
                    verify_items_lifetime(&inner_items, arena, semantic_ctx, source_manager, diagnostics);
                }
                _ => {
                    if let Err(e) = luna_semantic::lifetime::verify_before_codegen(decl, semantic_ctx, source_manager, arena) {
                        diagnostics.push(e.into_diagnostic());
                    }
                }
            }
        }
    }
}

/// Runs the full semantic pipeline (parse → resolve → typecheck → mono → lifetime verify)
/// and returns the collected diagnostics WITHOUT entering MVIR generation.
///
/// This function exists to mechanically verify the pipeline invariant:
///   "Semantic-invalid AST MUST NOT enter MVIR generation."
///
/// Returns `Err(diagnostics)` if any semantic errors are detected (same gate as `check()`),
/// or `Ok(())` if the program would have passed the semantic gate.
pub fn check_semantic_only(file_name: &str, input: String, options: &CompilerOptions) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    let file_id = session.source_manager.add_file(file_name.to_string(), input.clone());
    let lexer = Lexer::new(&input, file_id);
    let mut arena = AstArena::new();
    let mut parser = Parser::new(lexer, &mut arena, file_id);
    let mut items = parser.parse_file().map_err(|_| parser.diagnostics.clone())?;
    if !parser.diagnostics.is_empty() { return Err(parser.diagnostics); }
    let mut semantic_ctx = SemanticContext::new();

    let search_paths_buf: Vec<std::path::PathBuf> = options.search_paths.iter().map(std::path::PathBuf::from).collect();
    let sysroot = search_paths_buf
        .iter()
        .find(|p| p.join("libs").join("external").exists())
        .and_then(|p| crate::sysroot::Sysroot::from_root(p.clone()).ok())
        .or_else(|| crate::sysroot::Sysroot::discover(None).ok())
        .or_else(|| crate::sysroot::Sysroot::discover_for_test().ok())
        .unwrap_or_else(|| crate::sysroot::Sysroot::from_root(search_paths_buf.first().cloned().unwrap_or_else(|| std::path::PathBuf::from("."))).expect("Failed to initialize sysroot"));
    let mut driver_session = crate::session::DriverSession::new(sysroot, &mut session, options.search_paths.as_slice());
    driver_session.set_provider_bindings(&options.provider_bindings)?;

    if let Err(e) = driver_session.bootstrap_lang_contracts(&mut arena) {
        return Err(e.into_diagnostics());
    }

    let context = if options.is_sysroot_build {
        crate::resolution_context::ProviderResolutionContext::SysrootDependency
    } else {
        crate::resolution_context::ProviderResolutionContext::UserImport
    };
    crate::importer::resolve_imports(&mut items, &mut arena, &mut driver_session, context).map_err(|e| e)?;
    let registry = std::mem::take(&mut driver_session.registry);
    drop(driver_session);

    let mut attr_processor = luna_semantic::AttributeProcessor::new(&mut arena, &mut session.source_manager, file_id);
    let items = attr_processor.process_items(items).map_err(|e| e)?;

    registry.inject_into_ctx(&mut semantic_ctx);

    let mut resolver = Resolver::new(&mut semantic_ctx, &arena, &session.source_manager);
    resolver.register_macros(&items);
    if !semantic_ctx.diagnostics.is_empty() {
        return Err(semantic_ctx.diagnostics);
    }

    let mut macro_engine = luna_semantic::MacroEngine::new(
        &mut arena,
        &session.source_manager,
        file_id,
        &semantic_ctx.symbol_table,
        &semantic_ctx.tables,
    );
    let items = macro_engine.expand_items(items).map_err(|e| e)?;

    Resolver::new(&mut semantic_ctx, &arena, &session.source_manager).resolve_items(&items);
    let comptime_engine = luna_mvir::MvirComptimeEngine {
        max_steps: options.comptime_steps.unwrap_or(1_000_000),
        max_depth: options.comptime_depth.unwrap_or(512),
    };
    TypeChecker::new_with_engine(&mut semantic_ctx, &arena, &session.source_manager, &comptime_engine).typecheck_items(&items);

    let mut mono = luna_semantic::MonoCollector::new_with_source(&mut semantic_ctx, &arena, Some(&session.source_manager));
    mono.run(&items);
    let drop_glues = mono.drop_glues;
    let instantiated_functions = mono.instantiated.into_values().collect();
    semantic_ctx.instantiated_functions = instantiated_functions;
    semantic_ctx.drop_glue_instances = drop_glues;

    let mut diagnostics = semantic_ctx.diagnostics.clone();
    verify_items_lifetime(&items, &arena, &semantic_ctx, &session.source_manager, &mut diagnostics);
    let diagnostics: Vec<_> = diagnostics.into_iter().fold(Vec::new(), |mut acc, d| {
        if !acc.contains(&d) { acc.push(d); }
        acc
    });

    // This is the MVIR gate. We stop here — MVIR generation is NOT invoked.
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    Ok(())
}

pub fn check(file_name: &str, input: String, options: &CompilerOptions) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    check_with_session(&mut session, file_name, input, options)
}

/// Runs `check` against a caller-owned session so rendered diagnostics can
/// resolve their source spans through the same `SourceManager`.
pub fn check_with_session(session: &mut CompilerSession, file_name: &str, input: String, options: &CompilerOptions) -> Result<(), Vec<Diagnostic>> {
    let file_id = session.source_manager.add_file(file_name.to_string(), input.clone());
    let lexer = Lexer::new(&input, file_id);
    let mut arena = AstArena::new();
    let mut parser = Parser::new(lexer, &mut arena, file_id);
    let mut items = parser.parse_file().map_err(|_| parser.diagnostics.clone())?;
    if !parser.diagnostics.is_empty() { return Err(parser.diagnostics); }
    let mut semantic_ctx = SemanticContext::new();
    
    let search_paths_buf: Vec<std::path::PathBuf> = options.search_paths.iter().map(std::path::PathBuf::from).collect();
    let sysroot = search_paths_buf
        .iter()
        .find(|p| p.join("libs").join("external").exists())
        .and_then(|p| crate::sysroot::Sysroot::from_root(p.clone()).ok())
        .or_else(|| crate::sysroot::Sysroot::discover(None).ok())
        .or_else(|| crate::sysroot::Sysroot::discover_for_test().ok())
        .unwrap_or_else(|| crate::sysroot::Sysroot::from_root(search_paths_buf.first().cloned().unwrap_or_else(|| std::path::PathBuf::from("."))).expect("Failed to initialize sysroot"));
    let mut driver_session = crate::session::DriverSession::new(sysroot, session, options.search_paths.as_slice());
    driver_session.set_provider_bindings(&options.provider_bindings)?;
    
    let is_slice_authorized = if options.is_sysroot_build {
        driver_session.sysroot.get_canonical_provider_capabilities(std::path::Path::new(file_name))
            .map(|caps| caps.contains(&crate::sysroot_manifest::ProviderCapability::SliceInherentImpl))
            .unwrap_or(false)
    } else {
        false
    };
    semantic_ctx.is_slice_authorized = is_slice_authorized;

    if !options.is_sysroot_build {
        if let Err(e) = driver_session.bootstrap_lang_contracts(&mut arena) {
            return Err(e.into_diagnostics());
        }
    } else {
        semantic_ctx.allow_internal_lang_items = driver_session.sysroot
            .get_canonical_provider_capabilities(std::path::Path::new(file_name))
            .is_some();
    }
    
    let context = if options.is_sysroot_build {
        crate::resolution_context::ProviderResolutionContext::SysrootDependency
    } else {
        crate::resolution_context::ProviderResolutionContext::UserImport
    };
    crate::importer::resolve_imports(&mut items, &mut arena, &mut driver_session, context).map_err(|e| e)?;
    let registry = std::mem::take(&mut driver_session.registry);
    drop(driver_session);
    
    let mut attr_processor = luna_semantic::AttributeProcessor::new(&mut arena, &mut session.source_manager, file_id);
    let items = attr_processor.process_items(items).map_err(|e| e)?;

    registry.inject_into_ctx(&mut semantic_ctx);

    let mut resolver = Resolver::new(&mut semantic_ctx, &arena, &session.source_manager);
    resolver.register_macros(&items);
    if !semantic_ctx.diagnostics.is_empty() {
        return Err(semantic_ctx.diagnostics);
    }

    let mut macro_engine = luna_semantic::MacroEngine::new(
        &mut arena,
        &session.source_manager,
        file_id,
        &semantic_ctx.symbol_table,
        &semantic_ctx.tables,
    );
    let items = macro_engine.expand_items(items).map_err(|e| e)?;

    Resolver::new(&mut semantic_ctx, &arena, &session.source_manager).resolve_items(&items);
    let comptime_engine = luna_mvir::MvirComptimeEngine {
        max_steps: options.comptime_steps.unwrap_or(1_000_000),
        max_depth: options.comptime_depth.unwrap_or(512),
    };
    TypeChecker::new_with_engine(&mut semantic_ctx, &arena, &session.source_manager, &comptime_engine).typecheck_items(&items);

    let mut mono = luna_semantic::MonoCollector::new_with_source(&mut semantic_ctx, &arena, Some(&session.source_manager));
    mono.run(&items);
    let drop_glues = mono.drop_glues;
    let instantiated_functions = mono.instantiated.into_values().collect();
    semantic_ctx.instantiated_functions = instantiated_functions;
    semantic_ctx.drop_glue_instances = drop_glues;
    
    let mut diagnostics = semantic_ctx.diagnostics.clone();
    verify_items_lifetime(&items, &arena, &semantic_ctx, &session.source_manager, &mut diagnostics);
    let mut diagnostics: Vec<_> = diagnostics.into_iter().fold(Vec::new(), |mut acc, d| {
        if !acc.contains(&d) { acc.push(d); }
        acc
    });
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    
    let (module, mvir_diags) = luna_mvir::MvirGenerator::new(&arena, &semantic_ctx, &session.source_manager).generate(&items);
    if !mvir_diags.is_empty() {
        return Err(mvir_diags);
    }
    
    let mut interproc = luna_borrowck::interprocedural::InterproceduralContext::new(&semantic_ctx);
    seed_imported_raw_pointer_effects(&mut interproc, &module, &registry, &semantic_ctx);
    interproc.compute_summaries(&module);
    
    for function in module.functions {
        let (diags, _) = luna_borrowck::borrow_check_function(&function, &semantic_ctx, &interproc.summaries);
        diagnostics.extend(diags);
    }
    if diagnostics.is_empty() { if !options.quiet { println!("check passed"); } Ok(()) } else { Err(diagnostics) }
}

pub fn compile(file_name: &str, input: String, options: &CompilerOptions) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    compile_with_session(&mut session, file_name, input, options)
}

pub fn check_and_render(file_name: &str, input: String, options: &CompilerOptions) -> Result<(), String> {
    let mut session = CompilerSession::new();
    match check_with_session(&mut session, file_name, input, options) {
        Ok(()) => Ok(()),
        Err(diags) => Err(diags.iter().map(|d| d.render(&session.source_manager)).collect::<Vec<_>>().join("\n")),
    }
}

pub fn compile_and_render(file_name: &str, input: String, options: &CompilerOptions) -> Result<(), String> {
    let mut session = CompilerSession::new();
    match compile_with_session(&mut session, file_name, input, options) {
        Ok(()) => Ok(()),
        Err(diags) => Err(diags.iter().map(|d| d.render(&session.source_manager)).collect::<Vec<_>>().join("\n")),
    }
}

pub fn compile_with_session(session: &mut CompilerSession, file_name: &str, input: String, options: &CompilerOptions) -> Result<(), Vec<Diagnostic>> {
    if (options.emit_llib || options.emit_mlib)
        && options.output_path.as_ref().is_some_and(|path| path.ends_with(".mlib"))
    {
        return Err(vec![Diagnostic::error(
            "The legacy .mlib artifact format is read-only; emit canonical .llib instead".to_string(),
        ).with_code(luna_common::DiagnosticCode::InvalidArtifactOutput)]);
    }

    let file_id = session
        .source_manager
        .add_file(file_name.to_string(), input.clone());

    // Lexing phase
    let lexer = Lexer::new(&input, file_id);

    // Parsing phase
    let mut arena = AstArena::new();
    let mut parser = Parser::new(lexer, &mut arena, file_id);

    let file_result = parser.parse_file();
    let mut all_diagnostics = session.diagnostics.clone();
    all_diagnostics.extend(parser.diagnostics);

    if !all_diagnostics.is_empty() {
        return Err(all_diagnostics);
    }

    match file_result {
        Ok(items) => {
            if !options.quiet {
                println!("Parsed {} items", items.len());
                println!("AstArena Exprs count: {}", arena.exprs.len());
                println!("AstArena Stmts count: {}", arena.stmts.len());
                println!("AstArena Decls count: {}", arena.decls.len());
            }
            // Semantic phase
            let mut semantic_ctx = SemanticContext::new();
            
            let main_expr_end = arena.exprs.len() as u32;
            let main_decl_end = arena.decls.len() as u32;
            let main_pat_end = arena.pats.len() as u32;
            
            let mut items_mut = items.clone();
            
            let search_paths_buf: Vec<std::path::PathBuf> = options.search_paths.iter().map(std::path::PathBuf::from).collect();
            let sysroot = search_paths_buf
                .iter()
                .find(|p| p.join("libs").join("external").exists())
                .and_then(|p| crate::sysroot::Sysroot::from_root(p.clone()).ok())
                .or_else(|| crate::sysroot::Sysroot::discover(None).ok())
                .or_else(|| crate::sysroot::Sysroot::discover_for_test().ok())
                .unwrap_or_else(|| crate::sysroot::Sysroot::from_root(search_paths_buf.first().cloned().unwrap_or_else(|| std::path::PathBuf::from("."))).expect("Failed to initialize sysroot"));
            let mut driver_session = crate::session::DriverSession::new(sysroot, session, options.search_paths.as_slice());
            driver_session.set_provider_bindings(&options.provider_bindings)?;
            
            let base_name = std::path::Path::new(file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let provider_name = driver_session.sysroot
                .get_canonical_provider(std::path::Path::new(file_name))
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| base_name.to_string());
            semantic_ctx.current_provider_name = Some(provider_name.clone());
            let is_slice_authorized = if options.is_sysroot_build {
                driver_session.sysroot.get_canonical_provider_capabilities(std::path::Path::new(file_name))
                    .map(|caps| caps.contains(&crate::sysroot_manifest::ProviderCapability::SliceInherentImpl))
                    .unwrap_or(false)
            } else {
                false
            };
            semantic_ctx.is_slice_authorized = is_slice_authorized;

            if !options.is_sysroot_build {
                if let Err(e) = driver_session.bootstrap_lang_contracts(&mut arena) {
                    return Err(e.into_diagnostics());
                }
            } else {
                semantic_ctx.allow_internal_lang_items = driver_session.sysroot
                    .get_canonical_provider_capabilities(std::path::Path::new(file_name))
                    .is_some();
            }
            
            let context = if options.is_sysroot_build {
                crate::resolution_context::ProviderResolutionContext::SysrootDependency
            } else {
                crate::resolution_context::ProviderResolutionContext::UserImport
            };
            if let Err(e) = crate::importer::resolve_imports(&mut items_mut, &mut arena, &mut driver_session, context) {
                return Err(e);
            }
            
            let collected_objects = driver_session.collected_objects.clone();
            let registry = std::mem::take(&mut driver_session.registry);
            drop(driver_session);

            let mut attr_processor = luna_semantic::AttributeProcessor::new(&mut arena, &mut session.source_manager, file_id);
            let items_mut = attr_processor.process_items(items_mut).map_err(|e| e)?;

            registry.inject_into_ctx(&mut semantic_ctx);

            let mut resolver = Resolver::new(&mut semantic_ctx, &arena, &session.source_manager);
            resolver.register_macros(&items_mut);
            if !semantic_ctx.diagnostics.is_empty() {
                return Err(semantic_ctx.diagnostics);
            }

            let mut macro_engine = luna_semantic::MacroEngine::new(
                &mut arena,
                &session.source_manager,
                file_id,
                &semantic_ctx.symbol_table,
                &semantic_ctx.tables,
            );
            let items_mut = macro_engine.expand_items(items_mut).map_err(|e| e)?;

            let mut resolver = Resolver::new(&mut semantic_ctx, &arena, &session.source_manager);
            resolver.resolve_items(&items_mut);
            
            let comptime_engine = luna_mvir::MvirComptimeEngine {
                max_steps: options.comptime_steps.unwrap_or(1_000_000),
                max_depth: options.comptime_depth.unwrap_or(512),
            };
            let mut typechecker = TypeChecker::new_with_engine(&mut semantic_ctx, &arena, &session.source_manager, &comptime_engine);
            typechecker.typecheck_items(&items_mut);
            
            let mut mono = luna_semantic::MonoCollector::new_with_source(&mut semantic_ctx, &arena, Some(&session.source_manager));
            mono.run(&items_mut);
            let drop_glues = mono.drop_glues;
            let instantiated_functions = mono.instantiated.into_values().collect();
            semantic_ctx.instantiated_functions = instantiated_functions;
            semantic_ctx.drop_glue_instances = drop_glues;
            
            for diag in semantic_ctx.diagnostics.clone() {
                if !all_diagnostics.contains(&diag) {
                    all_diagnostics.push(diag);
                }
            }
            if !all_diagnostics.is_empty() {
                return Err(all_diagnostics);
            }
            
            if !options.quiet {
                println!("Resolved expr symbols: {}", semantic_ctx.tables.expr_symbols.len());
                println!("Resolved total symbols: {}", semantic_ctx.symbol_table.symbols.len());
                println!("Monomorphized instances: {}", semantic_ctx.instantiated_functions.len());
                
                // Print expression types
                println!("--- Expression Types ---");
                for (expr_id, ty_id) in &semantic_ctx.tables.expr_types {
                    let ty = semantic_ctx.types.get(*ty_id);
                    println!("Expr {:?}: {:?}", expr_id, ty);
                }
                println!("----------------------\n");
            }
            
            // MVIR phase
            verify_items_lifetime(&items_mut, &arena, &semantic_ctx, &session.source_manager, &mut all_diagnostics);
            if !all_diagnostics.is_empty() {
                return Err(all_diagnostics);
            }
            let mut generator = MvirGenerator::new(&arena, &semantic_ctx, &session.source_manager);
            generator.root_file_id = Some(file_id);
            let (mut module, mvir_diags) = generator.generate(&items_mut);
            if !mvir_diags.is_empty() {
                return Err(mvir_diags);
            }
            
            if !options.quiet {
                println!("\n--- Generated MVIR ---");
                println!("{}", print_module(&module));
                println!("----------------------\n");
            }
            if options.emit_mvir {
                let mvir_base = options
                    .output_path
                    .as_deref()
                    .map(std::path::Path::new)
                    .unwrap_or_else(|| std::path::Path::new(file_name));
                let mvir_file = mvir_base.with_extension("mvir");
                std::fs::write(&mvir_file, format!("{:#?}", module)).map_err(|error| {
                    vec![Diagnostic::error(format!(
                        "Failed to write MVIR output '{}': {}",
                        mvir_file.display(),
                        error
                    )).with_code(luna_common::DiagnosticCode::OutputWriteFailure)]
                })?;
            }
            
            // Borrow Checking Phase
            if !options.quiet {
                println!("\n--- Borrow Checker ---");
            }
            
            let mut interproc = luna_borrowck::interprocedural::InterproceduralContext::with_context(&semantic_ctx);
            seed_imported_raw_pointer_effects(&mut interproc, &module, &registry, &semantic_ctx);
            interproc.compute_summaries(&module);
            let summaries = interproc.summaries;
            
            let mut borrowck_errors = 0;
            for func in &mut module.functions {
                let (diagnostics, redundant_drops) = luna_borrowck::borrow_check_function(func, &semantic_ctx, &summaries);
                borrowck_errors += diagnostics.len();
                if !options.quiet {
                    for diag in &diagnostics {
                        println!("BorrowCk Error: {}", diag.message);
                    }
                }
                luna_borrowck::cleanup::eliminate_redundant_drops(func, &redundant_drops);
                all_diagnostics.extend(diagnostics);
            }

            if borrowck_errors != 0 {
                return Err(all_diagnostics);
            }

            if !options.quiet {
                if borrowck_errors == 0 {
                    println!("Borrow check passed!");
                }
                println!("----------------------\n");
            }
            
            // MVIR Verification (Pre-opt)
            if let Err(errs) = luna_optimizer::verify_module(&module) {
                if !options.quiet {
                    println!("--- Pre-Opt MVIR Verifier Error ---");
                    for e in errs {
                        println!("{}", e);
                    }
                    println!("-----------------------------------");
                }
                return Ok(());
            }

            // Optimization phase
            if !options.quiet {
                println!("\n--- Optimizer ---");
            }
            let mut pass_manager = luna_optimizer::PassManager::new();
            // Forward immutable numeric facts with the MVIR type handles.
            // Folding must not replace wrapping narrow arithmetic with an
            // untruncated host integer and then fold comparisons against it.
            let integer_ranges = semantic_ctx.types.get_all_types().into_iter().filter_map(|ty| {
                use luna_semantic::ty::{BuiltinType, SemanticType};
                let (bits, signed) = match semantic_ctx.types.get(semantic_ctx.types.resolve_inference(ty)) {
                    SemanticType::Primitive(BuiltinType::I8) => (8, true),
                    SemanticType::Primitive(BuiltinType::I16) => (16, true),
                    SemanticType::Primitive(BuiltinType::I32) => (32, true),
                    SemanticType::Primitive(BuiltinType::I64) => (64, true),
                    SemanticType::Primitive(BuiltinType::I128) => (128, true),
                    SemanticType::Primitive(BuiltinType::Isize) => (semantic_ctx.target_pointer_bits, true),
                    SemanticType::Primitive(BuiltinType::U8) => (8, false),
                    SemanticType::Primitive(BuiltinType::U16) => (16, false),
                    SemanticType::Primitive(BuiltinType::U32) => (32, false),
                    SemanticType::Primitive(BuiltinType::U64) => (64, false),
                    SemanticType::Primitive(BuiltinType::U128) => (128, false),
                    SemanticType::Primitive(BuiltinType::Usize) => (semantic_ctx.target_pointer_bits, false),
                    _ => return None,
                };
                Some((ty.0, bits, signed))
            });
            pass_manager.add_pass(Box::new(luna_optimizer::ConstantFolding::with_integer_ranges(integer_ranges)));
            pass_manager.add_pass(Box::new(luna_optimizer::DeadCodeElimination::new()));
            pass_manager.run(&mut module);
            if !options.quiet {
                println!("-----------------");
            }

            // MVIR Verification (Post-opt)
            if let Err(errs) = luna_optimizer::verify_module(&module) {
                if !options.quiet {
                    println!("--- Post-Opt MVIR Verifier Error ---");
                    for e in errs {
                        println!("{}", e);
                    }
                    println!("------------------------------------");
                }
                return Ok(());
            }

            // LLVM IR / Backend phase
            if !options.quiet {
                println!("\n--- Async Lowering & LLVM Backend ---");
            }
            
            // Async Lowering Phase (target specific, runs after MLib)
            async_lowering::lower_async(&mut module, &mut semantic_ctx);
            
            let config = TargetConfig::default();
            let llvm_context = inkwell::context::Context::create();
            let mut backend = LLVMBackend::new(&llvm_context, &module, &semantic_ctx, file_name, &config);
            
            if let Err(e) = backend.compile() {
                if !options.quiet { println!("Backend Error: {}", e); }
                return Err(vec![Diagnostic::error(format!("Backend Error: {}", e)).with_code(luna_common::DiagnosticCode::BackendInvariantViolation)]);
            }
            
            let base_name = std::path::Path::new(file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            // Save to file and compile — if output_path is specified, place .ll and .obj alongside it
            let (ll_file, obj_file, exe_file) = if let Some(ref out) = options.output_path {
                let out_path = std::path::Path::new(out);
                let parent = out_path.parent().unwrap_or_else(|| std::path::Path::new("."));
                let stem = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or(base_name);
                (
                    parent.join(format!("{}.ll", stem)).to_string_lossy().to_string(),
                    parent.join(format!("{}.obj", stem)).to_string_lossy().to_string(),
                    out.clone(),
                )
            } else {
                (
                    format!("{}.ll", base_name),
                    format!("{}.obj", base_name),
                    format!("{}.exe", base_name),
                )
            };
            
            let path_ll = std::path::Path::new(&ll_file);
            let path_obj = std::path::Path::new(&obj_file);
            
            if let Err(e) = backend.emit_ll(path_ll) {
                if !options.quiet { println!("Failed to emit .ll: {}", e); }
            } else {
                if !options.quiet { println!("Successfully wrote {}", ll_file); }
            }
            if !options.emit_llvm {
                let _ = std::fs::remove_file(path_ll);
            }
            
            if !options.quiet { println!("Target Triple: '{}'", config.triple); }
            
            // Emit .obj atomically using sibling temp + rename pattern
            let emit_obj_res = (|| -> Result<(), String> {
                let (tmp_file, tmp_obj_path, mut guard) = create_sibling_temp(path_obj, "obj")
                    .map_err(|e| format!("Failed to create temp .obj: {}", e))?;
                drop(tmp_file); // LLVM backend opens file path directly
                backend.emit_object(&tmp_obj_path, &config)
                    .map_err(|e| format!("Emit Object Error: {}", e))?;
                std::fs::rename(&tmp_obj_path, path_obj)
                    .map_err(|e| format!("Failed to finalize .obj: {}", e))?;
                guard.disarm();
                Ok(())
            })();
            if let Err(e) = emit_obj_res {
                if !options.quiet { println!("Failed to emit .obj: {}", e); }
                return Err(vec![Diagnostic::error(e).with_code(luna_common::DiagnosticCode::ObjectEmissionFailure)]);
            } else {
                if !options.quiet { println!("Successfully wrote {}", obj_file); }
            }

            // Canonical .llib generation phase with embedded ObjectCode.
            if options.emit_llib || options.emit_mlib {
                if !options.quiet {
                    println!("\n--- Serializing canonical LLib with ObjectCode ---");
                }
                let llib_file = options.output_path.as_ref()
                    .filter(|path| path.ends_with(".llib"))
                    .cloned()
                    .unwrap_or_else(|| format!("{}.llib", base_name));

                // Write to memory buffer first, then atomically publish
                // This prevents readers from observing partial/half-written artifacts
                let mut llib_buffer = Vec::new();
                let mut deps = vec![];
                let mut materialized_providers = std::collections::HashSet::new();
                for function in &module.functions {
                    if !function.is_extern {
                        if let Some(provider) = function.name.symbol_id
                            .and_then(|id| semantic_ctx.symbol_table.get_symbol(id).provider_id)
                        {
                            materialized_providers.insert(provider);
                        }
                    }
                }
                // Constant values can be embedded without a function instance.
                for instance in &semantic_ctx.instantiated_functions {
                    for expr in instance.expr_types.keys() {
                        if let Some(symbol) = semantic_ctx.tables.expr_symbols.get(expr) {
                            let symbol = semantic_ctx.symbol_table.get_symbol(*symbol);
                            if symbol.kind == luna_semantic::symbol::SymbolKind::Constant {
                                if let Some(provider) = symbol.provider_id {
                                    materialized_providers.insert(provider);
                                }
                            }
                        }
                    }
                }
                for (id, interface) in &registry.interfaces {
                    if interface.name != provider_name {
                        let dep_entry = luna_llib::format::DependencyEntry {
                            provider_name: interface.name.clone(),
                            interface_fingerprint: interface.interface_fingerprint,
                            execution_fingerprint: if semantic_ctx.tables.evaluated_comptime
                                || materialized_providers.contains(id)
                            {
                                interface.execution_fingerprint
                            } else {
                                None
                            },
                        };
                        deps.push(dep_entry);
                    }
                }

                let manifest = luna_llib::Manifest {
                    identity: luna_llib::ArtifactIdentity {
                        package_id: "".to_string(),
                        version: "0.1.0".to_string(),
                        module_id: provider_name.clone(),
                        artifact_id: "".to_string(),
                    },
                    target: luna_llib::TargetContract {
                        target_triple: config.triple.clone(),
                        object_format: "ELF".to_string(),
                        abi: "".to_string(),
                        pointer_width: 64,
                        endianness: "".to_string(),
                    },
                    dependencies: luna_llib::format::DependencyTable {
                        deps,
                        native_deps: vec![],
                    },
                    object_metadata: None,
                    provenance: luna_llib::format::Provenance {
                        source_fingerprint: luna_llib::format::Fingerprint([0; 32]),
                        compiler_version: "0.1.0".to_string(),
                        codegen_options: "".to_string(),
                        interface_fingerprint: luna_llib::format::Fingerprint([0; 32]),
                        execution_fingerprint: None,
                    },
                    export_table: None,
                };

                let main_provider_id = luna_semantic::symbol::ProviderId(0);
                let ranges = crate::registry::ArenaRanges {
                    exprs: 0..main_expr_end,
                    decls: 0..main_decl_end,
                    pats: 0..main_pat_end,
                };
                let interface = crate::registry::ModuleRegistry::extract_interface_from_ctx(provider_name.clone(), main_provider_id, &semantic_ctx, &ranges);
                let builder = crate::metadata_builder::MetadataBuilder::new(&registry, &interface, &summaries);
                let semantic_metadata = Some(builder.build());

                let obj_bytes = std::fs::read(path_obj).ok();

                match luna_llib::LlibWriter::write_module(&module, &arena, &items, &input, manifest, semantic_metadata.as_ref(), obj_bytes.as_deref(), &mut llib_buffer) {
                    Ok(_) => {
                        // Atomically publish the complete artifact
                        // Use sibling temp file + rename pattern with RAII cleanup
                        let canonical_path = std::path::Path::new(&llib_file);
                        let ext_suffix = "llib";
                        match create_sibling_temp(canonical_path, ext_suffix) {
                            Ok((mut tmp_file, tmp_path, mut guard)) => {
                                use std::io::Write;
                                if let Err(e) = tmp_file.write_all(&llib_buffer) {
                                    if !options.quiet { println!("Failed to write temp artifact: {}", e); }
                                } else if let Err(e) = tmp_file.flush() {
                                    if !options.quiet { println!("Failed to flush temp artifact: {}", e); }
                                } else {
                                    drop(tmp_file); // Close handle before rename on Windows
                                    match std::fs::rename(&tmp_path, canonical_path) {
                                        Ok(_) => {
                                            guard.disarm();
                                            if !options.quiet { println!("Successfully wrote {}", llib_file); }
                                        }
                                        Err(e) => {
                                            if !options.quiet { println!("Failed to publish artifact: {}", e); }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if !options.quiet { println!("Failed to create temp artifact: {}", e); }
                            }
                        }
                    }
                    Err(e) => {
                        if !options.quiet { println!("Failed to serialize library: {}", e); }
                    }
                }
                if !options.quiet {
                    println!("---------------------------\n");
                }
            }
            
            if !options.no_link {
                let mut all_objs = vec![path_obj.to_path_buf()];
                for ext_obj in &collected_objects {
                    if ext_obj.exists() && !all_objs.contains(ext_obj) {
                        all_objs.push(ext_obj.clone());
                    }
                }
                if !options.quiet {
                    println!("Linking {} object file(s) to {}...", all_objs.len(), exe_file);
                }
                match luna_backend::link_objs_to_exe(&all_objs, std::path::Path::new(&exe_file)) {
                    Ok(_) => {
                        if !options.quiet { println!("Build successful: {}", exe_file); }
                    }
                    Err(e) => {
                        if !options.quiet { println!("Link failed: {}", e); }
                        return Err(vec![Diagnostic::error(format!("Link Error: {}", e)).with_code(luna_common::DiagnosticCode::LinkerFailure)]);
                    }
                }
            }
            
        }
        Err(_e) => {
            if !options.quiet { println!("Failed to parse file."); }
        }
    }

    if all_diagnostics.is_empty() {
        Ok(())
    } else {
        Err(all_diagnostics)
    }
}

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// RAII guard for temporary artifacts.
/// Ensures the temp file is removed on scope exit unless explicitly disarmed after successful publication.
pub(crate) struct TempArtifactGuard {
    path: std::path::PathBuf,
    armed: bool,
}

impl TempArtifactGuard {
    pub(crate) fn new(path: std::path::PathBuf) -> Self {
        Self { path, armed: true }
    }

    pub(crate) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TempArtifactGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Create a unique sibling temporary file in the same parent directory as `canonical_path`.
/// Guaranteed collision-free via `create_new(true)` and PID + atomic counter.
/// The temp file does NOT have the canonical artifact extension (e.g. `.llib` or `.obj`),
/// preventing resolver discovery from accidentally picking it up.
pub(crate) fn create_sibling_temp(
    canonical_path: &std::path::Path,
    ext_suffix: &str,
) -> Result<(std::fs::File, std::path::PathBuf, TempArtifactGuard), std::io::Error> {
    let parent = canonical_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem = canonical_path.file_stem().and_then(|s| s.to_str()).unwrap_or("artifact");
    let pid = std::process::id();
    loop {
        let counter = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp_name = format!("{}.{}.tmp.{}.{}", stem, ext_suffix, pid, counter);
        let tmp_path = parent.join(tmp_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
        {
            Ok(file) => {
                let guard = TempArtifactGuard::new(tmp_path.clone());
                return Ok((file, tmp_path, guard));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

pub fn test_backend_compile(
    module: &luna_mvir::Module,
    semantic_ctx: &SemanticContext,
) -> Result<(), luna_backend::BackendError> {
    let config = TargetConfig::default();
    let llvm_context = inkwell::context::Context::create();
    let mut backend = LLVMBackend::new(&llvm_context, module, semantic_ctx, "test", &config);
    backend.compile()
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_atomic_publication_successful_replacement() {
        let temp_dir = std::env::temp_dir().join("luna_pub_test_success");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let canonical = temp_dir.join("box.llib");
        // Write initial complete content
        std::fs::write(&canonical, b"OLD COMPLETE BYTES").unwrap();

        // Perform publication using create_sibling_temp + rename
        let (mut tmp_file, tmp_path, mut guard) = create_sibling_temp(&canonical, "llib").unwrap();
        assert!(tmp_path.exists());
        assert!(!tmp_path.to_string_lossy().ends_with(".llib"));

        tmp_file.write_all(b"NEW COMPLETE BYTES").unwrap();
        tmp_file.flush().unwrap();
        drop(tmp_file); // Close handle on Windows before rename

        std::fs::rename(&tmp_path, &canonical).unwrap();
        guard.disarm();

        // Canonical has new content
        let content = std::fs::read(&canonical).unwrap();
        assert_eq!(content, b"NEW COMPLETE BYTES");

        // Temp is gone
        assert!(!tmp_path.exists());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_atomic_publication_failure_cleans_temp_and_preserves_old() {
        let temp_dir = std::env::temp_dir().join("luna_pub_test_fail");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let canonical = temp_dir.join("vec.llib");
        std::fs::write(&canonical, b"ORIGINAL PRESERVED CONTENT").unwrap();

        let tmp_path_copy;
        {
            let (mut tmp_file, tmp_path, _guard) = create_sibling_temp(&canonical, "llib").unwrap();
            tmp_path_copy = tmp_path.clone();
            assert!(tmp_path.exists());

            tmp_file.write_all(b"PARTIAL").unwrap();
            // Simulate error/abort before rename: _guard drops while armed
        }

        // After guard drops on error, temp must be automatically deleted
        assert!(!tmp_path_copy.exists(), "Temp artifact must be removed on failure");

        // Canonical remains unchanged
        let content = std::fs::read(&canonical).unwrap();
        assert_eq!(content, b"ORIGINAL PRESERVED CONTENT");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sibling_temp_discovery_isolation() {
        let temp_dir = std::env::temp_dir().join("luna_pub_test_isolation");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let canonical = temp_dir.join("hashmap.llib");
        let (_file, tmp_path, mut guard) = create_sibling_temp(&canonical, "llib").unwrap();

        // 1. Same parent directory
        assert_eq!(tmp_path.parent(), canonical.parent());

        // 2. Extension is NOT .llib
        let ext = tmp_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_ne!(ext, "llib", "Temp file must not end in .llib extension");

        guard.disarm();
        let _ = std::fs::remove_file(&tmp_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}



