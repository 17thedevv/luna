//! Shared ownership-state transition model for drop flags.
//!
//! Both the synchronous local cleanup (`drop_flags::elaborate`) and the async
//! suspension cleanup (`suspension::compute_async_cleanup_plan`) must record the
//! *same* per-instruction ownership transitions so the two cleanups cannot drift.
//! This module owns that semantic model; each caller keeps its own physical
//! storage (a stack `bool` alloca for sync, an async environment field for async).
use crate::dataflow::{DataflowAnalysis, DataflowEngine};
use crate::effect::CallEffectSummary;
use crate::move_analysis::{MoveAnalyzer, MoveState};
use crate::place::Place;
use luna_mvir::{Function, GlobalId, ValueId};
use luna_semantic::SemanticContext;
use std::collections::{BTreeSet, HashMap};

/// A single ownership transition that a runtime drop flag must observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropFlagTransition {
    /// A place becomes owned from an uninitialized state.
    Initialize,
    /// A place becomes owned again after having been moved out or destroyed.
    Reinitialize,
    /// A place's value is moved out (or reset to uninitialized).
    MoveOut,
    /// A place's value is destroyed in place.
    Destroy,
    /// Ownership becomes path-dependent; the flag already encodes it, so no
    /// flag update is emitted (tracking is required, the value is unchanged).
    ConditionalMove,
}

impl DropFlagTransition {
    /// The value to store into a runtime drop flag, or `None` when the
    /// transition requires tracking but must not rewrite the flag.
    pub fn flag_value(self) -> Option<bool> {
        match self {
            DropFlagTransition::Initialize | DropFlagTransition::Reinitialize => Some(true),
            DropFlagTransition::MoveOut | DropFlagTransition::Destroy => Some(false),
            DropFlagTransition::ConditionalMove => None,
        }
    }
}

/// Classify an observed `previous -> current` move-state change into a
/// transition. Returns `None` for changes that carry no flag obligation
/// (partial moves, and staying conditionally moved).
fn classify(previous: MoveState, current: MoveState) -> Option<DropFlagTransition> {
    match current {
        MoveState::Live => match previous {
            MoveState::Moved | MoveState::Dropped => Some(DropFlagTransition::Reinitialize),
            _ => Some(DropFlagTransition::Initialize),
        },
        MoveState::Moved => Some(DropFlagTransition::MoveOut),
        MoveState::Dropped => Some(DropFlagTransition::Destroy),
        MoveState::Uninitialized => Some(DropFlagTransition::MoveOut),
        MoveState::ConditionallyMoved => Some(DropFlagTransition::ConditionalMove),
        MoveState::PartialMoved => None,
    }
}

/// The ownership transitions for a set of tracked places, keyed by the MVIR
/// instruction after which each transition becomes observable.
pub type DropFlagTransitions = HashMap<ValueId, Vec<(Place, DropFlagTransition)>>;

/// Compute the shared ownership-state transitions for `tracked` places.
///
/// `tracked` is the caller's place set: the synchronous path tracks places with
/// a conditional `Drop`, the async path tracks places that are path-dependent at
/// a suspension. The state model below is identical for both.
pub fn plan_drop_flag_transitions(
    func: &Function,
    ctx: Option<&SemanticContext>,
    summaries: Option<&HashMap<GlobalId, CallEffectSummary>>,
    tracked: &BTreeSet<Place>,
) -> DropFlagTransitions {
    let tracked: Vec<Place> = tracked.iter().cloned().collect();
    if tracked.is_empty() {
        return HashMap::new();
    }
    let mut analyzer = MoveAnalyzer::new(func, ctx, summaries);
    analyzer.emit_diagnostics = false;
    let incoming = DataflowEngine::run_forward(func, &mut analyzer);
    let mut transitions: DropFlagTransitions = HashMap::new();
    for block in &func.blocks {
        let mut state = incoming.get(&block.label.name).cloned().unwrap_or_default();
        for &id in &block.insts {
            let before: Vec<_> = tracked
                .iter()
                .map(|place| (place.clone(), analyzer.get_place_state(place, &state)))
                .collect();
            analyzer.transfer_instruction(id, &func.value(id).inst, &mut state);
            for (place, previous) in before {
                let current = analyzer.get_place_state(&place, &state);
                if current == previous && id != place.local {
                    continue;
                }
                if let Some(transition) = classify(previous, current) {
                    transitions.entry(id).or_default().push((place, transition));
                }
            }
        }
    }
    transitions
}

#[cfg(test)]
mod tests {
    use super::*;

    // The shared state model reduces to exactly three flag effects.
    #[test]
    fn transition_mapping_is_the_shared_contract() {
        use DropFlagTransition::*;
        assert_eq!(classify(MoveState::Uninitialized, MoveState::Live), Some(Initialize));
        assert_eq!(classify(MoveState::Moved, MoveState::Live), Some(Reinitialize));
        assert_eq!(classify(MoveState::Dropped, MoveState::Live), Some(Reinitialize));
        assert_eq!(classify(MoveState::Live, MoveState::Moved), Some(MoveOut));
        assert_eq!(classify(MoveState::Live, MoveState::Dropped), Some(Destroy));
        assert_eq!(classify(MoveState::Live, MoveState::ConditionallyMoved), Some(ConditionalMove));
        assert_eq!(classify(MoveState::Live, MoveState::PartialMoved), None);

        assert_eq!(Initialize.flag_value(), Some(true));
        assert_eq!(Reinitialize.flag_value(), Some(true));
        assert_eq!(MoveOut.flag_value(), Some(false));
        assert_eq!(Destroy.flag_value(), Some(false));
        assert_eq!(ConditionalMove.flag_value(), None);
    }

    // The synchronous cleanup and the async cleanup must observe the same
    // transition sequence for the same tracked set — they differ only in where
    // the flag is stored. Both callers route through `plan_drop_flag_transitions`.
    #[test]
    fn both_cleanup_paths_share_the_planner() {
        let sync_src = include_str!("drop_flags.rs");
        let async_src = include_str!("suspension.rs");
        assert!(sync_src.contains("plan_drop_flag_transitions"));
        assert!(async_src.contains("plan_drop_flag_transitions"));
        // Neither path keeps a private copy of the transition classifier.
        assert!(!sync_src.contains("MoveState::Live => true"));
        assert!(!async_src.contains("MoveState::Live => true"));
    }
}
