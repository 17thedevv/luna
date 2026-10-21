//! Read-only source inventory using Luna's parser, without semantic/codegen claims.
use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for argument in env::args_os().skip(1) {
        let path = PathBuf::from(argument);
        let text = fs::read_to_string(&path)?;
        let file = luna_common::ids::FileId(0);
        let mut arena = luna_ast::AstArena::new();
        let mut parser =
            luna_parser::Parser::new(luna_lexer::Lexer::new(&text, file), &mut arena, file);
        let result = parser.parse_file();
        if result.is_err() || !parser.diagnostics.is_empty() {
            return Err(format!("{}: {:?}", path.display(), parser.diagnostics).into());
        }
        let items = result.unwrap();
        files.push(serde_json::json!({
            "file": path,
            "items": items,
            "declarations": arena.decls,
            "types": arena.types,
            "expressions": arena.exprs,
        }));
    }
    serde_json::to_writer(io::stdout().lock(), &files)?;
    io::stdout().flush()?;
    Ok(())
}
