use crate::dataflow::{DataflowAnalysis, DataflowEngine};
use crate::drop_flag_plan::plan_drop_flag_transitions;
use crate::move_analysis::{MoveAnalyzer, MoveState, MoveStateData};
use crate::place::Place;
use luna_mvir::{Function, Instruction, ValueId};
use luna_semantic::SemanticContext;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct SuspensionState {
    pub initialized_places: HashSet<Place>,
    pub moved_places: HashSet<Place>,
    pub live_places: HashSet<Place>,
    /// Places that may or may not be owned at this suspension (moved on one
    /// path, live on another). They must be destroyed through a runtime
    /// ownership flag, never unconditionally.
    pub conditional_places: HashSet<Place>,
}

#[derive(Debug, Clone)]
pub struct AsyncStateMap {
    pub initial_state: SuspensionState,
    pub await_states: HashMap<ValueId, SuspensionState>,
}

fn extract_live_places(current_state: &MoveStateData) -> (HashSet<Place>, HashSet<Place>, HashSet<Place>, HashSet<Place>) {
    let mut initialized = HashSet::new();
    let mut moved = HashSet::new();
    let mut live_raw = HashSet::new();
    let mut conditional = HashSet::new();

    for (place, state) in &current_state.places {
        match state {
            MoveState::Live => {
                live_raw.insert(place.clone());
                initialized.insert(place.clone());
            }
            MoveState::PartialMoved => {
                initialized.insert(place.clone());
            }
            MoveState::ConditionallyMoved => {
                // Ownership is path-dependent: must go through a runtime flag,
                // not an unconditional drop.
                conditional.insert(place.clone());
                initialized.insert(place.clone());
            }
            MoveState::Moved | MoveState::Dropped => {
                moved.insert(place.clone());
            }
            MoveState::Uninitialized => {}
        }
    }

    // Filter out descendant places if their ancestor is already Live (to avoid double drops).
    let mut live = HashSet::new();
    for p in &live_raw {
        let has_live_ancestor = live_raw.iter().any(|other| other != p && p.is_descendant_of(other));
        if !has_live_ancestor {
            live.insert(p.clone());
        }
    }

    (initialized, moved, live, conditional)
}

pub fn compute_suspension_states(
    func: &Function,
    semantic_ctx: Option<&SemanticContext>,
) -> AsyncStateMap {
    let mut move_analyzer = MoveAnalyzer::new(func, semantic_ctx, None);
    move_analyzer.emit_diagnostics = false;

    let block_states = DataflowEngine::run_forward(func, &mut move_analyzer);
    let mut await_states = HashMap::new();
    let mut initial_state_opt = None;

    if let Some(entry_block) = func.blocks.first() {
        let mut current_state = block_states.get(&entry_block.label.name).cloned().unwrap_or_default();

        // 1. Compute initial state (State 0)
        for val_id in &entry_block.insts {
            let val_data = &func.values[val_id.0 as usize];
            if !matches!(val_data.inst, Instruction::Alloca) {
                break;
            }
            move_analyzer.transfer_instruction(*val_id, &val_data.inst, &mut current_state);
        }

        let (init_initialized, init_moved, init_live, init_conditional) = extract_live_places(&current_state);
        initial_state_opt = Some(SuspensionState {
            initialized_places: init_initialized,
            moved_places: init_moved,
            live_places: init_live,
            conditional_places: init_conditional,
        });

        // 2. Compute await states
        for block in &func.blocks {
            let mut current_state = block_states.get(&block.label.name).cloned().unwrap_or_default();
            for val_id in &block.insts {
                let val_data = &func.values[val_id.0 as usize];
                if matches!(val_data.inst, Instruction::Await { .. }) {
                    let (initialized, moved, live, conditional) = extract_live_places(&current_state);

                    await_states.insert(
                        *val_id,
                        SuspensionState {
                            initialized_places: initialized,
                            moved_places: moved,
                            live_places: live,
                            conditional_places: conditional,
                        },
                    );
                }

                // Advance state
                move_analyzer.transfer_instruction(*val_id, &val_data.inst, &mut current_state);
            }
        }
    }

    let default_init = SuspensionState {
        initialized_places: HashSet::new(),
        moved_places: HashSet::new(),
        live_places: HashSet::new(),
        conditional_places: HashSet::new(),
    };

    AsyncStateMap {
        initial_state: initial_state_opt.unwrap_or(default_init),
        await_states,
    }
}

/// Async cleanup needs runtime ownership flags only for places whose ownership
/// is path-dependent at some suspension (ConditionallyMoved). This returns those
/// places plus the per-instruction ownership transitions that must be recorded
/// in the environment so cancellation can consult the flags.
pub struct AsyncCleanupPlan {
    pub conditional_places: HashSet<Place>,
    pub transitions: HashMap<ValueId, Vec<(Place, bool)>>,
}

pub fn compute_async_cleanup_plan(
    func: &Function,
    semantic_ctx: Option<&SemanticContext>,
) -> AsyncCleanupPlan {
    let states = compute_suspension_states(func, semantic_ctx);
    let mut conditional_places: HashSet<Place> = HashSet::new();
    conditional_places.extend(states.initial_state.conditional_places.iter().cloned());
    for state in states.await_states.values() {
        conditional_places.extend(state.conditional_places.iter().cloned());
    }
    if conditional_places.is_empty() {
        return AsyncCleanupPlan { conditional_places, transitions: HashMap::new() };
    }

    // Record ownership transitions for the flagged places through the shared
    // planner; only the flag storage (an async environment field) is async
    // specific.
    let tracked: BTreeSet<Place> = conditional_places.iter().cloned().collect();
    let raw = plan_drop_flag_transitions(func, semantic_ctx, None, &tracked);
    let mut transitions: HashMap<ValueId, Vec<(Place, bool)>> = HashMap::new();
    for (id, events) in raw {
        for (place, transition) in events {
            if let Some(initialized) = transition.flag_value() {
                transitions.entry(id).or_default().push((place, initialized));
            }
        }
    }

    AsyncCleanupPlan { conditional_places, transitions }
}
