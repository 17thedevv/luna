use mellis_common::CompilerSession;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mellis-rs <file.ms>");
        process::exit(1);
    }

    let file_path = &args[1];
    let source = fs::read_to_string(file_path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", file_path, err);
        process::exit(1);
    });

    // In Phase 2, we setup a session here to render diagnostics if needed.
    let mut session = CompilerSession::new();
    session
        .source_manager
        .add_file(file_path.clone(), source.clone());

    match mellis_driver::compile(file_path, &source) {
        Ok(_) => {
            println!("Compiled successfully.");
            process::exit(0);
        }
        Err(diagnostics) => {
            for diag in diagnostics {
                eprintln!("{}", diag.render(&session.source_manager));
            }
            process::exit(1);
        }
    }
}
