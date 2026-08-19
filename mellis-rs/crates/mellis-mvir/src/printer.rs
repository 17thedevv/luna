use crate::mvir::*;

pub fn print_module(module: &Module) -> String {
    let mut out = String::new();
    for func in &module.functions {
        out.push_str(&format!("fn {}() {{\n", func.name.name));
        for block in &func.blocks {
            out.push_str(&format!("{}:\n", block.label.name));
            for inst in &block.instructions {
                out.push_str(&format!("  {}\n", print_instruction(inst)));
            }
            if let Some(term) = &block.terminator {
                out.push_str(&format!("  {}\n", print_terminator(term)));
            } else {
                out.push_str("  <missing terminator>\n");
            }
        }
        out.push_str("}\n\n");
    }
    out
}

fn print_instruction(inst: &Instruction) -> String {
    match inst {
        Instruction::Alloca { dest, ty: _ } => {
            format!("{} = alloca", dest.name)
        }
        Instruction::Store { ptr, value } => {
            format!("store {} -> {}", print_operand(value), print_operand(ptr))
        }
        Instruction::Load { dest, ptr, ty: _ } => {
            format!("{} = load {}", dest.name, print_operand(ptr))
        }
        Instruction::Add { dest, left, right, ty: _ } => {
            format!("{} = add {}, {}", dest.name, print_operand(left), print_operand(right))
        }
        Instruction::Sub { dest, left, right, ty: _ } => {
            format!("{} = sub {}, {}", dest.name, print_operand(left), print_operand(right))
        }
        Instruction::Mul { dest, left, right, ty: _ } => {
            format!("{} = mul {}, {}", dest.name, print_operand(left), print_operand(right))
        }
        Instruction::Call { dest, callee, args, ret_ty: _ } => {
            let args_str: Vec<String> = args.iter().map(print_operand).collect();
            let dest_str = if let Some(d) = dest { format!("{} = ", d.name) } else { "".to_string() };
            format!("{}call {}({})", dest_str, print_operand(callee), args_str.join(", "))
        }
    }
}

fn print_terminator(term: &Terminator) -> String {
    match term {
        Terminator::Ret { value } => {
            if let Some(val) = value {
                format!("ret {}", print_operand(val))
            } else {
                "ret void".to_string()
            }
        }
        Terminator::Br { target } => {
            format!("br {}", target.name)
        }
        Terminator::CondBr { condition, true_target, false_target } => {
            format!("condbr {}, {}, {}", print_operand(condition), true_target.name, false_target.name)
        }
        Terminator::Unreachable => {
            "unreachable".to_string()
        }
    }
}

fn print_operand(op: &Operand) -> String {
    match op {
        Operand::Local(loc) => loc.name.clone(),
        Operand::Global(glb) => glb.name.clone(),
        Operand::Number(n) => n.clone(),
        Operand::Boolean(b) => b.to_string(),
    }
}
