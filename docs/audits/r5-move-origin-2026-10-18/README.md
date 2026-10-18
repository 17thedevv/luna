# R5 move-origin joins

Status: compiler repair under validation; candidate remains `c085449b`.

All eight release lanes passed at implementation `3a72f946`, including full
workspace triage/final on both Windows and Ubuntu (each 1414 passed, 0 failed,
1 ignored). The [previous report](../r5-linux-native-link-2026-10-18/README.md)
preserves those results and their scope.

Final review then found a real DIAG-3/9 gap: after nested branches move the same
value at different source sites, E3001 still rejects the invalid use but can
lose its structural antecedent label. The [original reducer and check/build
outputs](evidence/finding.json) preserve that failure. Correct rejection alone
does not discharge the related-span requirement.

## Root cause and repair

`MoveAnalyzer::merge` previously joined origins only when the ownership state
changed, and deleted the origin when two incoming spans differed. Diagnostic
provenance could therefore disappear at a join or depend on predecessor order.

The repair joins provenance independently of ownership state, retaining the
earliest valid incoming source span under a total location/context ordering.
An origin-only change participates in dataflow convergence. Ownership state
transitions themselves are unchanged.

The same provenance review found a stale-field case: after a field is moved,
reinitialized and then the entire aggregate is moved, the diagnostic could
still blame the obsolete first field move. The [additional reducer](evidence/field-origin-finding.json)
records the original-binary result and the identical move-analysis source at
52ef3711 and 3a72f946. Actual move/drop/live transitions now discard obsolete
subtree origins. Internal state compaction retains valid field-specific
antecedents and chooses a deterministic aggregate origin; provenance-only
subplaces participate in joins even when their ownership states were compacted.

Compiler Change

    Capability: Structural move antecedents survive CFG joins deterministically.
    Why stdlib exposed it: Release diagnostic review exercised ordinary moves;
        this reducer uses no standard-library type.
    Why it is generic: Place provenance is joined for arbitrary user values.
    User-defined type benefiting: Record in move_origin_join fixtures.
    Tests: Public CLI negative/control matrix for nested, equal-state and loop
        joins, including a generic artifact-only provider; internal predecessor
        permutations and fixed-point assertions; borrowck regressions.
    New intrinsic/lang_item?: NO. Versions remain 25/9/5/2.
    Stdlib-specific branch?: NO.

## Validation plan

- Check and build each negative in isolated source/artifact roots: one E3001,
  the actual invalid-use primary line and earliest valid move-origin line, no
  executable, repeatable locations and check/build parity.
- Execute valid consumption and reinitialization controls natively.
- Verify the current aggregate move after field reinitialization, and preserve
  the precise field move when several dropped fields compact into one state.
- Check join commutativity across all predecessor permutations, propagation
  when only provenance changes, and convergence to a fixed point.
- Re-run affected borrowck regressions, select a new immutable implementation
  pin, and reacquire every release lane. No `3a72f946` PASS is carried forward.

The first CLI test attempt passed all source cases and four direct artifact
cases. Its imported-generic assertion expected a `.ln` display path, while the
source-free artifact correctly reported the `.llib` path with the same primary
and antecedent line numbers. The test now checks the proper provider path for
each mode; the source and origin requirements are unchanged.

SKILL IMPACT: REFINEMENT to `luna-language-capability-validation`: diagnostic
origin claims now explicitly require nested/equal-state joins, convergence,
reinitialization and preservation of valid field causes during compaction.

## Selected replacement

Implementation pin: `67146195fb025951ca51d54365fd3a47ef9544e6`. The [local preflight](evidence/local-preflight.json) passes: seven negatives in two provider modes, each checked twice and built (42 negative commands), correct primary/antecedent lines, no executable, and valid native controls in both modes. All 65 affected borrowck tests pass, including predecessor permutations and fixed-point/field metadata invariants. This preflight reuses the existing Windows runtime and is not exact-pin release qualification.

Documentation regeneration is idempotent at the selected implementation (230 documents, 30 pages, 1807 checked local links). The next Ubuntu normal harness explicitly rejects a dirty final checkout and preserves final identity; hosted Windows now also saves its final identity. New full-workspace gates require the move-origin CLI suite in addition to the E3004 and native-math suites. All eight lanes must complete again.
