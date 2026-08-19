use mellis_common::{CompilerSession, Diagnostic};
use mellis_lexer::Lexer;

pub fn compile(file_name: &str, input: &str) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    let file_id = session
        .source_manager
        .add_file(file_name.to_string(), input.to_string());

    // Lexing phase
    let lexer = Lexer::new(input, file_id);
    let mut tokens = Vec::new();

    for token in lexer {
        tokens.push(token);
        // Ngừng lexing nếu gặp lỗi
        if token.kind == mellis_lexer::TokenKind::Error {
            session.diagnostics.push(
                Diagnostic::error("Lexer encountered an invalid token.").with_span(token.span),
            );
            break;
        }
    }

    if session.diagnostics.is_empty() {
        // Debug print tokens for now
        for t in &tokens {
            println!("{:?}", t);
        }
        Ok(())
    } else {
        Err(session.diagnostics.clone())
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
