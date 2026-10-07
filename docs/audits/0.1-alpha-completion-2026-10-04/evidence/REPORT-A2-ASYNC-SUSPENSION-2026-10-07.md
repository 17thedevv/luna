<!-- luna-doc-role: evidence -->

# A2 report — async suspension-state cleanup

Branch `a2-async-suspended-cleanup`, base `04b273a9`, implementation `6e93f2a7`.

## Original design flaw

`async_lowering` generated the cancellation `*_drop` function by iterating
`live_places` from `compute_suspension_states` and calling `emit_drop_for_place`,
which emits an **unconditional** `Drop`. Suspension state (which await the future
is stopped at) and ownership are **separate axes**: at the same suspension state
an object can be owned on one branch and already moved on another. `state` alone
therefore cannot decide cleanup.

## Failure modes

- **ConditionallyMoved** — a place moved on only one path before a suspension was
  part of `live_places` and destroyed unconditionally → **double-drop** on the
  moved branch. Pinned by the A2-PROOF test, which FAILED before the fix.
- **PartialMoved** — excluded from the unconditional set; the remaining
  initialized subplaces are represented as their own places, so they stay
  covered (PROOF-5 passed).

## Implementation boundary

- `suspension.rs`: `ConditionallyMoved` removed from `live_places` and collected
  into `conditional_places`; `compute_async_cleanup_plan` returns the flagged
  places plus the per-instruction ownership transitions they need.
- async environment: runtime ownership flag fields **appended after** existing
  fields (ordinals unchanged); kickoff initializes parameter-owned places to
  true.
- poll/resume: ownership transitions (initialize → true, move → false, reinit →
  true) are recorded into the flags.
- drop_fn: definitely-live places dropped directly; conditionally-moved places
  guarded by `Load(flag)` / `CondBr` / `Drop`; finally `HeapFree(env)`.
- `state == COMPLETED` keeps the free-env-only path (no re-drop after normal
  completion).

## Protocol

compiler **21**, metadata **9**, MVIR **5**, format **2**. Only the compiler
protocol was bumped: older artifacts carry async bodies with the previous cleanup
semantics and must not be trusted.

## Evidence

- `a2_suspension_cleanup_tests`: 3 passed (conditional guarded-drop; partial
  covered; guard present in the generated drop_fn).
- `p1c_tests` (async × drop/cancel): 15 passed.
- `cargo test --workspace --no-fail-fast`: **exit 0 — 206 targets, 1334 passed /
  0 failed / 1 ignored** (candidate was 1329 / 0 / 1).

## Remaining (explicit debt, not hidden)

- **A2-FU1**: extract a shared `DropFlagPlan` / `DropFlagTransition` abstraction so
  sync (alloca) and async (env field) cleanup cannot drift. The async path
  currently mirrors the transition logic in `suspension.rs`. Semantics are
  correct today; this is maintainability / anti-drift.
- **A2-FU2**: complete the CLI source/fresh-artifact async cleanup matrix
  (borrow-only, generic owned value, projected reinit, cancel at state #1/#2).
  Suspend-state cases are currently covered by the driver-level test.

## Verdict

**A2 CLOSED / CONFORMANT IN TESTED SCOPE.** Release remains BLOCKED; next
correctness blocker: A3.
