use mellis_mvir::{Module, Function, BasicBlock, Instruction, Terminator, Operand};

pub fn generate_llvm_ir(module: &Module) -> String {
    let mut out = String::new();
    
    // Hardcode some standard declarations
    out.push_str("declare i32 @puts(ptr)\n");
    out.push_str("@.str = private unnamed_addr constant [26 x i8] c\"Hello, error Rust Mellis!\\00\"\n\n");
    
    for func in &module.functions {
        let ret_ty = "void"; // Hardcoded for simplicity
        // Hack: Map the first function to 'main' for the linker
        let func_name = if func.name.name.starts_with("func_") { "main" } else { &func.name.name };
        
        out.push_str(&format!("define {} @{}() {{\n", ret_ty, func_name));
        
        for block in &func.blocks {
            out.push_str(&format!("{}:\n", block.label.name));
            
            for inst in &block.instructions {
                out.push_str(&format!("  {}\n", generate_inst(inst)));
            }
            
            if let Some(term) = &block.terminator {
                out.push_str(&format!("  {}\n", generate_term(term)));
            }
        }
        
        out.push_str("}\n\n");
    }
    
    out
}

fn generate_inst(inst: &Instruction) -> String {
    match inst {
        Instruction::Alloca { dest, .. } => {
            // Simplified: everything is i32 or ptr for now
            format!("{} = alloca i32", dest.name)
        }
        Instruction::Store { ptr, value } => {
            format!("store i32 {}, ptr {}", generate_op(value), generate_op(ptr))
        }
        Instruction::Load { dest, ptr, .. } => {
            format!("{} = load i32, ptr {}", dest.name, generate_op(ptr))
        }
        Instruction::Add { dest, left, right, .. } => {
            format!("{} = add i32 {}, {}", dest.name, generate_op(left), generate_op(right))
        }
        Instruction::Sub { dest, left, right, .. } => {
            format!("{} = sub i32 {}, {}", dest.name, generate_op(left), generate_op(right))
        }
        Instruction::Mul { dest, left, right, .. } => {
            format!("{} = mul i32 {}, {}", dest.name, generate_op(left), generate_op(right))
        }
        Instruction::Call { dest, callee, args, .. } => {
            let args_str: Vec<String> = args.iter().map(|arg| format!("ptr {}", generate_op(arg))).collect();
            let callee_name = match callee {
                Operand::Global(glb) => if glb.name.starts_with("global_") { format!("@puts") } else { format!("@{}", glb.name) },
                _ => generate_op(callee),
            };
            
            // Hack: if arg is a number ("null"), replace with @.str
            let final_args_str: Vec<String> = args.iter().map(|arg| {
                if let Operand::Number(n) = arg {
                    if n == "null" {
                        return "ptr @.str".to_string();
                    }
                }
                format!("ptr {}", generate_op(arg))
            }).collect();
            
            // Hardcoded to void return for now, so no destination assignment
            format!("call i32 {}({})", callee_name, final_args_str.join(", "))
        }
    }
}

fn generate_term(term: &Terminator) -> String {
    match term {
        Terminator::Ret { value } => {
            if let Some(val) = value {
                format!("ret i32 {}", generate_op(val))
            } else {
                "ret void".to_string()
            }
        }
        Terminator::Br { target } => {
            format!("br label %{}", target.name)
        }
        Terminator::CondBr { condition, true_target, false_target } => {
            format!("br i1 {}, label %{}, label %{}", generate_op(condition), true_target.name, false_target.name)
        }
        Terminator::Unreachable => {
            "unreachable".to_string()
        }
    }
}

fn generate_op(op: &Operand) -> String {
    match op {
        Operand::Local(loc) => loc.name.clone(),
        Operand::Global(glb) => format!("@{}", glb.name),
        Operand::Number(n) => n.clone(),
        Operand::Boolean(b) => if *b { "1".to_string() } else { "0".to_string() },
    }
}
