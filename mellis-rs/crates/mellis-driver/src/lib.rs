use mellis_common::{CompilerSession, Diagnostic, Span};

pub fn compile(file_name: &str, input: &str) -> Result<(), Vec<Diagnostic>> {
    let mut session = CompilerSession::new();
    let file_id = session
        .source_manager
        .add_file(file_name.to_string(), input.to_string());

    // Test Diagnostic rendering
    if input.contains("error") {
        let span = Span {
            file_id,
            start: input.find("error").unwrap() as u32,
            end: input.find("error").unwrap() as u32 + 5,
        };
        session
            .diagnostics
            .push(Diagnostic::error("Found the word 'error'").with_span(span));
    }

    if session.diagnostics.is_empty() {
        Ok(())
    } else {
        // Trả về kèm theo Diagnostics
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_pipeline() {
        let result = compile("test.ms", "fn main() {}");
        assert!(result.is_ok());

        let result = compile("test.ms", "fn error() {}");
        assert!(result.is_err());
    }
}
