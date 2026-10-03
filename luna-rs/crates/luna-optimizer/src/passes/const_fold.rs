use luna_mvir::{Function, Instruction, Operand};
use crate::pass::Pass;

pub struct ConstantFolding {
    integer_ranges: Option<std::collections::HashMap<u32, (u32, bool)>>,
}

impl Pass for ConstantFolding {
    fn name(&self) -> &'static str {
        "ConstantFolding"
    }

    fn run_on_function(&mut self, func: &mut Function) -> bool {
        let mut changed = false;

        for i in 0..func.values.len() {
            let val = &func.values[i];
            let new_inst = match &val.inst {
                Instruction::Add { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        l.checked_add(r).map(|n| Instruction::Assign(Operand::Number(n.to_string())))
                    } else { None }
                }
                Instruction::Neg { value } => {
                    if let Some(v) = Self::resolve_const(func, value) {
                        v.checked_neg().map(|n| Instruction::Assign(Operand::Number(n.to_string())))
                    } else { None }
                }
                Instruction::Sub { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        l.checked_sub(r).map(|n| Instruction::Assign(Operand::Number(n.to_string())))
                    } else { None }
                }
                Instruction::Mul { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        l.checked_mul(r).map(|n| Instruction::Assign(Operand::Number(n.to_string())))
                    } else { None }
                }
                Instruction::Div { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        if r != 0 {
                            l.checked_div(r).map(|n| Instruction::Assign(Operand::Number(n.to_string())))
                        } else { None }
                    } else { None }
                }
                Instruction::Rem { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        if r != 0 {
                            l.checked_rem(r).map(|n| Instruction::Assign(Operand::Number(n.to_string())))
                        } else { None }
                    } else { None }
                }
                Instruction::Eq { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        Some(Instruction::Assign(Operand::Boolean(l == r)))
                    } else { None }
                }
                Instruction::NotEq { left, right } => {
                    if let (Some(l), Some(r)) = (Self::resolve_const(func, left), Self::resolve_const(func, right)) {
                        Some(Instruction::Assign(Operand::Boolean(l != r)))
                    } else { None }
                }
                _ => None,
            };

            let new_inst = new_inst.filter(|inst| {
                if let Instruction::Assign(Operand::Number(number)) = inst {
                    if let Some(ranges) = &self.integer_ranges {
                        let Some(&(bits, signed)) = ranges.get(&val.ty.0) else { return false; };
                        let Ok(number) = number.parse::<i128>() else { return false; };
                        if signed {
                            bits == 128 || (number >= -(1i128 << (bits - 1)) && number < (1i128 << (bits - 1)))
                        } else {
                            number >= 0 && (bits == 128 || (number as u128) < (1u128 << bits))
                        }
                    } else { true }
                } else { true }
            });
            if let Some(inst) = new_inst {
                func.values[i].inst = inst;
                changed = true;
            }
        }

        changed
    }
}

impl ConstantFolding {
    pub fn new() -> Self {
        Self { integer_ranges: None }
    }

    /// Immutable numeric facts indexed by the MVIR type handles in this session.
    /// The optimizer does not inspect the semantic phase's internal type table.
    pub fn with_integer_ranges(ranges: impl IntoIterator<Item = (u32, u32, bool)>) -> Self {
        let integer_ranges = ranges.into_iter().map(|(ty, bits, signed)| (ty, (bits, signed))).collect();
        Self { integer_ranges: Some(integer_ranges) }
    }
    
    // Attempt to resolve an operand to a constant integer
    fn resolve_const(func: &Function, op: &Operand) -> Option<i64> {
        match op {
            Operand::Number(n) => n.parse::<i64>().ok(),
            Operand::Value(id) => {
                // Without a semantic type table, only fold the non-negative
                // i64 subset where signed and unsigned arithmetic agree. A
                // negative spelling may instead encode u128 bits (e.g. -1).
                match &func.value(*id).inst {
                    Instruction::Assign(Operand::Number(number)) => number.parse::<i64>().ok().filter(|&n| n >= 0),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
