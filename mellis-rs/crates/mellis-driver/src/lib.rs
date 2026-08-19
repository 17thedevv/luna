use mellis_ast::AstArena;
use mellis_common::{CompilerSession, Diagnostic};
use mellis_lexer::Lexer;
use mellis_parser::Parser;
use mellis_semantic::{SemanticContext, Resolver, TypeChecker};
use mellis_mvir::{MvirGenerator, print_module};
use mellis_backend::{generate_llvm_ir, compile_ll_to_exe};

pub fn compile(file_name: &str, input: &str) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    let file_id = session
        .source_manager
        .add_file(file_name.to_string(), input.to_string());

    // Lexing phase
    let lexer = Lexer::new(input, file_id);

    // Parsing phase
    let mut arena = AstArena::new();
    let mut parser = Parser::new(lexer, &mut arena, file_id);

    let file_result = parser.parse_file();
    let parser_diagnostics = parser.diagnostics;

    match file_result {
        Ok(items) => {
            println!("Parsed {} items", items.len());
            println!("AstArena Exprs count: {}", arena.exprs.len());
            println!("AstArena Stmts count: {}", arena.stmts.len());
            println!("AstArena Decls count: {}", arena.decls.len());
            
            // Semantic phase
            let mut semantic_ctx = SemanticContext::new();
            
            let mut resolver = Resolver::new(&mut semantic_ctx, &arena);
            resolver.resolve_items(&items);
            
            let mut typechecker = TypeChecker::new(&mut semantic_ctx, &arena);
            typechecker.typecheck_items(&items);
            
            println!("Resolved symbols: {}", semantic_ctx.tables.expr_symbols.len());
            
            // Print expression types
            println!("--- Expression Types ---");
            for (expr_id, ty_id) in &semantic_ctx.tables.expr_types {
                let ty = semantic_ctx.types.get(*ty_id);
                println!("Expr {:?}: {:?}", expr_id, ty);
            }
            println!("----------------------\n");
            
            // MVIR phase
            let generator = MvirGenerator::new(&arena, &semantic_ctx);
            let module = generator.generate(&items);
            
            println!("\n--- Generated MVIR ---");
            println!("{}", print_module(&module));
            println!("----------------------\n");
            
            // LLVM IR / Backend phase
            let llvm_ir = generate_llvm_ir(&module);
            println!("\n--- Generated LLVM IR ---");
            println!("{}", llvm_ir);
            println!("-------------------------\n");
            
            // Save to file and compile
            let ll_file = format!("{}.ll", file_name);
            let obj_file = format!("{}.obj", file_name);
            let exe_file = format!("{}.exe", file_name.replace(".ms", ""));
            
            std::fs::write(&ll_file, &llvm_ir).expect("Failed to write .ll file");
            
            println!("Compiling to {}...", exe_file);
            match compile_ll_to_exe(&ll_file, &obj_file, &exe_file) {
                Ok(_) => println!("Build successful: {}", exe_file),
                Err(e) => println!("Build failed: {}", e),
            }
            
        }
        Err(_) => {
            println!("Failed to parse file.");
        }
    }

    let mut all_diagnostics = session.diagnostics;
    all_diagnostics.extend(parser_diagnostics);

    if all_diagnostics.is_empty() {
        Ok(())
    } else {
        Err(all_diagnostics)
    }
}

pub fn render_diagnostics(input: &str, diagnostics: &[Diagnostic]) -> String {
    let mut session = CompilerSession::new();
    session
        .source_manager
        .add_file("dummy.ms".to_string(), input.to_string());

    diagnostics
        .iter()
        .map(|d| d.render(&session.source_manager))
        .collect::<Vec<_>>()
        .join("\n")
}
