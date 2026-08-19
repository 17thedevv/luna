use mellis_ast::AstArena;
use mellis_common::{CompilerSession, Diagnostic};
use mellis_lexer::Lexer;
use mellis_parser::Parser;

pub fn compile(file_name: &str, input: &str) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    let file_id = session.source_manager.add_file(file_name.to_string(), input.to_string());
    
    // Lexing phase
    let lexer = Lexer::new(input, file_id);
    
    // Parsing phase
    let mut arena = AstArena::new();
    let mut parser = Parser::new(lexer, &mut arena, file_id);

    let expr_result = parser.parse_expr();
    let parser_diagnostics = parser.diagnostics;

    // Dùng parse_expr() để thử nghiệm parse một biểu thức duy nhất (tạm thời)
    match expr_result {
        Ok(expr_id) => {
            println!("Parsed Expr ID: {:?}", expr_id);
            println!("AstArena Exprs count: {}", arena.exprs.len());
            println!("First Expr: {:?}", arena.exprs[expr_id.0 as usize]);
        }
        Err(_) => {
            println!("Failed to parse expression.");
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
    session.source_manager.add_file("dummy.ms".to_string(), input.to_string());
    
    diagnostics.iter().map(|d| d.render(&session.source_manager)).collect::<Vec<_>>().join("\n")
}
