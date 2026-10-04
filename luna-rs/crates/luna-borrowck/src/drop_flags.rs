//! Lower whole-local conditional cleanup without weakening definite use checks.
use crate::{
    dataflow::{DataflowAnalysis, DataflowEngine},
    effect::CallEffectSummary,
    move_analysis::{MoveAnalyzer, MoveState},
    place::Place,
};
use luna_mvir::{
    BasicBlock, Function, GlobalId, Instruction, LabelId, Operand, Terminator, ValueData, ValueId,
};
use luna_semantic::{SemanticContext, SemanticTypeId};
use std::collections::{BTreeMap, HashMap, HashSet};

fn append(func: &mut Function, inst: Instruction, ty: SemanticTypeId) -> ValueId {
    let id = ValueId(func.values.len() as u32);
    func.values.push(ValueData::new(inst, ty, None));
    id
}

fn label(used: &mut HashSet<String>, stem: &str) -> LabelId {
    let mut index = used.len();
    loop {
        let name = format!("{stem}_{index}");
        if used.insert(name.clone()) {
            return LabelId { name };
        }
        index += 1;
    }
}

pub(crate) fn elaborate(
    func: &mut Function,
    ctx: &SemanticContext,
    summaries: &HashMap<GlobalId, CallEffectSummary>,
) -> HashSet<ValueId> {
    if func.is_extern {
        return HashSet::new();
    }
    let mut analyzer = MoveAnalyzer::new(func, Some(ctx), Some(summaries));
    let incoming = DataflowEngine::run_forward(func, &mut analyzer);
    let mut conditional = HashMap::new();
    let mut locals = BTreeMap::new();
    for block in &func.blocks {
        let mut state = incoming.get(&block.label.name).cloned().unwrap_or_default();
        for &id in &block.insts {
            if let Instruction::Drop {
                value: Operand::Value(value),
                ..
            } = &func.value(id).inst
            {
                if let Some(place) = analyzer.values_to_places.get(value) {
                    if place.projections.is_empty()
                        && matches!(func.value(place.local).inst, Instruction::Alloca)
                        && analyzer.get_place_state(place, &state) == MoveState::ConditionallyMoved
                    {
                        conditional.insert(id, place.local);
                        locals.insert(place.local, ());
                    }
                }
            }
            analyzer.transfer_instruction(id, &func.value(id).inst, &mut state);
        }
    }
    if conditional.is_empty() {
        return HashSet::new();
    }

    // Record the same ownership transitions that move analysis uses, at each
    // runtime operation, rather than deriving initialization from a CFG join.
    let mut updates: HashMap<ValueId, Vec<(ValueId, bool)>> = HashMap::new();
    for block in &func.blocks {
        let mut state = incoming.get(&block.label.name).cloned().unwrap_or_default();
        for &id in &block.insts {
            let before: Vec<_> = locals
                .keys()
                .map(|&local| (local, analyzer.get_place_state(&Place::new(local), &state)))
                .collect();
            analyzer.transfer_instruction(id, &func.value(id).inst, &mut state);
            for (local, previous) in before {
                let current = analyzer.get_place_state(&Place::new(local), &state);
                if current != previous || id == local {
                    let initialized = match current {
                        MoveState::Live => true,
                        MoveState::Moved | MoveState::Dropped | MoveState::Uninitialized => false,
                        _ => continue,
                    };
                    updates.entry(id).or_default().push((local, initialized));
                }
            }
        }
    }
    drop(analyzer);
    let bool_ty = ctx.types.bool_id();
    let mut flags = BTreeMap::new();
    let mut entry_flags = Vec::new();
    for local in locals.keys() {
        let flag = append(func, Instruction::Alloca, bool_ty);
        entry_flags.push(flag);
        entry_flags.push(append(
            func,
            Instruction::Store {
                ptr: Operand::Value(flag),
                value: Operand::Boolean(false),
            },
            bool_ty,
        ));
        flags.insert(*local, flag);
    }
    let mut labels: HashSet<_> = func
        .blocks
        .iter()
        .map(|block| block.label.name.clone())
        .collect();
    let original = std::mem::take(&mut func.blocks);
    let mut lowered = Vec::new();
    for (block_index, block) in original.into_iter().enumerate() {
        let mut current = BasicBlock {
            label: block.label,
            insts: Vec::new(),
            terminator: None,
        };
        if block_index == 0 {
            current.insts.append(&mut entry_flags);
        }
        for id in block.insts {
            if let Some(local) = conditional.get(&id) {
                let flag = flags[local];
                let live = append(
                    func,
                    Instruction::Load {
                        ptr: Operand::Value(flag),
                    },
                    bool_ty,
                );
                current.insts.push(live);
                let drop_block = label(&mut labels, "drop_if_initialized");
                let continuation = label(&mut labels, "after_conditional_drop");
                current.terminator = Some(Terminator::CondBr {
                    condition: Operand::Value(live),
                    true_target: drop_block.clone(),
                    false_target: continuation.clone(),
                });
                lowered.push(current);
                lowered.push(BasicBlock {
                    label: drop_block,
                    insts: vec![id],
                    terminator: Some(Terminator::Br {
                        target: continuation.clone(),
                    }),
                });
                current = BasicBlock {
                    label: continuation,
                    insts: Vec::new(),
                    terminator: None,
                };
            } else {
                current.insts.push(id);
            }
            if let Some(events) = updates.get(&id) {
                for &(local, initialized) in events {
                    current.insts.push(append(
                        func,
                        Instruction::Store {
                            ptr: Operand::Value(flags[&local]),
                            value: Operand::Boolean(initialized),
                        },
                        bool_ty,
                    ));
                }
            }
        }
        current.terminator = block.terminator;
        lowered.push(current);
    }
    func.blocks = lowered;
    conditional.into_keys().collect()
}
