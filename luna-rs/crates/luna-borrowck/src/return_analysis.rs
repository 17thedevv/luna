use luna_common::{Diagnostic, DiagnosticCode};
use luna_mvir::{Function, Operand, Terminator};
use std::collections::{HashMap, HashSet};

pub fn analyze_returns(func: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    if func.blocks.is_empty() {
        return diagnostics;
    }
    
    // Map block label names to their index in the blocks array
    let mut label_to_idx = HashMap::new();
    for (i, block) in func.blocks.iter().enumerate() {
        label_to_idx.insert(block.label.name.clone(), i);
    }
    
    let mut visited = HashSet::new();
    let mut stack = vec![0]; // entry block is usually 0
    
    while let Some(block_idx) = stack.pop() {
        if !visited.insert(block_idx) {
            continue;
        }
        
        if block_idx >= func.blocks.len() { continue; }
        let block = &func.blocks[block_idx];
        if let Some(terminator) = &block.terminator {
            match terminator {
                Terminator::Br { target } => {
                    if let Some(&idx) = label_to_idx.get(&target.name) {
                        stack.push(idx);
                    }
                }
                Terminator::CondBr { condition, true_target, false_target } => {
                    // Literal/materialized boolean conditions have only one
                    // executable successor. In particular, an endless loop's
                    // synthetic exit is not a reachable missing return.
                    if !matches!(condition, Operand::Boolean(false)) {
                        if let Some(&idx) = label_to_idx.get(&true_target.name) {
                            stack.push(idx);
                        }
                    }
                    if !matches!(condition, Operand::Boolean(true)) {
                        if let Some(&idx) = label_to_idx.get(&false_target.name) {
                            stack.push(idx);
                        }
                    }
                }
                Terminator::MissingReturn => {
                    // DIAG-2: anchor the diagnostic at the last instruction of
                    // the block whose control flow falls off the end, never at a
                    // fabricated location.
                    let mut diag = Diagnostic::error("Function expects a return value, but control flow reaches the end without returning.".to_string()).with_code(DiagnosticCode::MissingReturnValue);
                    if let Some(span) = block
                        .insts
                        .last()
                        .and_then(|value| func.values.get(value.0 as usize))
                        .and_then(|value| value.span.clone())
                    {
                        diag = diag.with_span(span);
                    }
                    diagnostics.push(diag);
                }
                Terminator::Ret { .. } | Terminator::Unreachable => {}
            }
        }
    }
    
    diagnostics
}
