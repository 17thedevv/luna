<!-- luna-doc-role: evidence -->

# A2-FU1 report — shared DropFlagPlan / DropFlagTransition

Branch `a2-fu1-drop-flag-plan` (from candidate `f4b7c0de`).
Refactor only — no semantics change.

## Problem (anti-drift)

The synchronous cleanup (`drop_flags::elaborate`) and the async suspension
cleanup (`suspension::compute_async_cleanup_plan`) each **duplicated** the same
per-instruction ownership-state transition loop:

```
Live                          -> flag = true
Moved | Dropped | Uninitialized -> flag = false
ConditionallyMoved | PartialMoved -> skip
```

Two copies of the same state model can drift.

## Change

New module `luna-borrowck/src/drop_flag_plan.rs` owns the semantic model:

- `DropFlagTransition { Initialize, Reinitialize, MoveOut, Destroy, ConditionalMove }`
  with `flag_value() -> Option<bool>` (Initialize/Reinitialize -> true,
  MoveOut/Destroy -> false, ConditionalMove -> none);
- `plan_drop_flag_transitions(func, ctx, summaries, tracked)` performs the shared
  dataflow + transition classification.

Both paths route through it and keep their **own physical storage**:

| Path | Flag storage |
|---|---|
| sync | a per-place `bool` alloca (`drop_flags::elaborate`) |
| async | the future environment field (`suspension::compute_async_cleanup_plan`) |

The shared abstraction deliberately keys on `Place` + transition (logical
identity), not on a `ValueId`/env-field index, so the sync and async layouts stay
decoupled. `AsyncCleanupPlan.transitions` still exposes `(Place, bool)` so the
async lowering consumer is untouched.

## Evidence that the model is shared

- `drop_flag_plan::tests::transition_mapping_is_the_shared_contract` — the
  classify + flag-value table.
- `drop_flag_plan::tests::both_cleanup_paths_share_the_planner` — asserts both
  `drop_flags.rs` and `suspension.rs` reference `plan_drop_flag_transitions` and
  that neither keeps its own classifier.

## Guards

`generic_drop_cli` 1 (closure_env_* and A3 partial-aggregate reducers), 
`a2_suspension_cleanup_tests` 3, `async_cleanup_cli` 1,
`async_provider_export_cli` 2, `luna-llib` 0 failed, `luna-borrowck` 0 failed.

## Not in scope

closure ownership semantics; async frame layout beyond consuming the shared plan;
MVIR changes; serialized artifact changes; C4-FU1; general cleanup-subsystem
cleanup.

## Versioning

No bump: internal refactor — the emitted ownership transitions are unchanged, so
generated MVIR and artifacts are unchanged (compiler22 / metadata9 / MVIR5 /
format2).

## Verdict

**A2-FU1: DONE.** The shared `DropFlagPlan`/`DropFlagTransition` model is used by
both the synchronous and async cleanup paths; no duplicated transition logic
remains.
