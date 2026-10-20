//! D2 freeze — expression span precision.
//!
//! For four expression errors, assert (1) the exact code, (2) the primary span
//! points at the offending token/expression (smallest useful span, not the whole
//! statement), and (3) the rendered line/column and caret are stable between
//! `check` and `build`. Identity is code + span; wording is not asserted.
#[path = "support/stdlib.rs"]
mod support;

use std::fs;
use support::{render, workspace_root, ProviderModes};

/// The rendered block of the first diagnostic carrying `code`.
fn diagnostic_block(stderr: &str, code: &str) -> String {
    let mut block = Vec::new();
    let mut inside = false;
    for line in stderr.lines() {
        if line.starts_with("error[") {
            if inside {
                break;
            }
            inside = line.contains(code);
        } else if line.starts_with("warning[") || line.starts_with("note:") {
            if inside {
                break;
            }
        }
        if inside {
            block.push(line);
        }
    }
    block.join("\n")
}

fn expected_column(line: &str, token: &str) -> usize {
    line.find(token).unwrap() + 1
}

#[test]
fn expression_error_spans_are_precise_and_stable() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/diagnostic_spans");
    // (fixture, code, offending line, offending token)
    let matrix = [
        ("deref_scalar", "E2002", 4usize, "value"),
        ("neg_bool", "E2012", 4, "value"),
        ("tuple_scalar", "E2003", 4, "value"),
        ("index_scalar", "E2003", 4, "value"),
        ("char_invalid", "E2026", 3, "55296"),
    ];
    let mut failures = Vec::new();
    for (name, code, line_number, token) in matrix {
        let fixture: std::path::PathBuf = fixtures.join(format!("{name}.ln"));
        let source = fs::read_to_string(&fixture).unwrap();
        let offending = source.lines().nth(line_number - 1).unwrap();
        let column = expected_column(offending, token);
        for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
            let check = modes.check(root, &fixture);
            let (exe, build) = modes.build(root, &fixture, &format!("d2_{name}_{mode}"));
            if exe.exists() {
                failures.push(format!("{mode}/{name}: published an executable"));
            }
            let mut blocks = Vec::new();
            for (command, output) in [("check", &check), ("build", &build)] {
                if output.status.code() != Some(1) {
                    failures.push(format!("{mode}/{name}/{command}: {}", render(output)));
                    continue;
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                let block = diagnostic_block(&stderr, code);
                if block.is_empty() {
                    failures.push(format!("{mode}/{name}/{command}: missing error[{code}]: {}", render(output)));
                    continue;
                }
                let location = format!(":{line_number}:{column}");
                if !block.lines().any(|l| l.trim_start().starts_with("-->") && l.contains(&location)) {
                    failures.push(format!(
                        "{mode}/{name}/{command}: span is not at {location}: {}",
                        render(output)
                    ));
                }
                let caret = block.lines().find(|l| l.contains('^'));
                match caret {
                    None => failures.push(format!("{mode}/{name}/{command}: no caret: {}", render(output))),
                    Some(line) => {
                        let width = line.chars().filter(|c| *c == '^').count();
                        if width != token.chars().count() {
                            failures.push(format!(
                                "{mode}/{name}/{command}: caret width {width} != token width {}: {}",
                                token.chars().count(),
                                render(output)
                            ));
                        }
                    }
                }
                blocks.push(block);
            }
            if blocks.len() == 2 && blocks[0] != blocks[1] {
                failures.push(format!("{mode}/{name}: check/build span differs"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
