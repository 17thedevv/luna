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

## Superseded origin-repair pin

Implementation pin: `67146195fb025951ca51d54365fd3a47ef9544e6`. The [local preflight](evidence/local-preflight.json) passes: seven negatives in two provider modes, each checked twice and built (42 negative commands), correct primary/antecedent lines, no executable, and valid native controls in both modes. All 65 affected borrowck tests pass, including predecessor permutations and fixed-point/field metadata invariants. This preflight reuses the existing Windows runtime and is not exact-pin release qualification.

Documentation regeneration is idempotent at the selected implementation (230 documents, 30 pages, 1807 checked local links). The next Ubuntu normal harness explicitly rejects a dirty final checkout and preserves final identity; hosted Windows now also saves its final identity. New full-workspace gates require the move-origin CLI suite in addition to the E3004 and native-math suites. All eight lanes must complete again.

## Historical acquisition for 67146195

Harness: `6eab700e5fe211a407b6a7f9aebb16a150a4cbf3`. Authenticated GitHub job observations confirm native acquisition has started:

- [Ubuntu normal / ASan / comptime](https://github.com/17thedevv/luna/actions/runs/39099784852)
- [Ubuntu workspace triage then final](https://github.com/17thedevv/luna/actions/runs/39099784909)
- [Windows normal / workspace triage then final](https://github.com/17thedevv/luna/actions/runs/39099784890)

These links identify acquisition for the superseded pin. [State and job IDs](qualification-state.json) and [remaining requirements](REQUIREMENTS.md) retain the full release scope.

## Additional identity/order audit before final closure

Pin 67146195 is superseded during qualification. Its [verified runtime ASan result](evidence/pin-67146195/ubuntu-asan-verification.json) remains historical. The [DIAG-4 reducer](evidence/dedup-finding.json) demonstrates two distinct forbidden partial moves collapsed by message-based dedup. The [DIAG-6 reducer](evidence/ordering-finding.json) demonstrates multi-function error order changing across six processes.

The complete follow-up uses a shared typed source-event identity, replaces message/full-object equality in Borrowck, TypeChecker and Driver dedup, and normalizes diagnostics at public Driver boundaries by source location and code. Duplicate source events retain the union of structural cause spans. Global/unanchored and uncoded diagnostics are preserved. Internal ordering uses source identity rather than message text or function/hash traversal. Regression checks distinct identical-wording E3002 events and multi-process E3001 ordering in both provider modes, in addition to origin joins and reinitialization.

Compiler Change

    Capability: Distinct source events survive dedup; public diagnostic results
        follow source-path/range/code order, retaining structural cause spans.
    Why stdlib exposed it: Final release review used ordinary user-defined
        aggregates and functions, without relying on a standard-library type.
    Why it is generic: Common identity and Driver result normalization apply
        to arbitrary source diagnostics; analyzer dedup uses typed identities.
    User-defined type benefiting: Record with Drop in distinct_partial_moves.
    Tests: Common identity/order/related-union/idempotence invariants; repeated
        source/artifact CLI checks and build; existing diagnostic regressions.
    New intrinsic/lang_item?: NO. Versions remain 25/9/5/2.
    Stdlib-specific branch?: NO.

The old workflows could not be cancelled with available tooling: the connector
has no cancellation operation and the isolated browser is signed out. They may
finish as historical evidence; their results cannot qualify the replacement.

The [repair source review](evidence/identity-order-source-review.json) records
the inspected boundaries and file hashes. Local preflight passes 15 common and
65 borrowck tests, plus the expanded CLI matrix in both provider modes. All nine existing diagnostic regression tests also pass. The
[complete local preflight](evidence/identity-order-preflight.json) preserves
compressed logs and hashes; it reuses the Windows runtime and is not exact-pin
release qualification.

SKILL IMPACT: REFINEMENT to `luna-language-capability-validation`: identity and
order claims now require identical-wording distinct events and multi-function
errors across independent processes. Related skills were checked for overlap;
the update adds regression guidance, with no semantic or freeze authority.

## Selected identity/order replacement

Implementation pin: `d3d0ffe8d6495aee1f56257a1db817ce4107e992`. All 90 local preflight tests pass;
documentation generation is idempotent and validation checks 1815 local links
without errors. This is not release qualification. The three workflows now
select this immutable SHA for all eight mandatory lanes, including separate
workspace triage and dependent final passes on both hosts. Candidate remains
`c085449b`; no prior-pin result carries forward.

Qualification harness: `bf35297b7797215eabf01572b0531a78ab62511c`.
Authenticated job observations confirm setup has started for
[Ubuntu normal/ASan/comptime](https://github.com/17thedevv/luna/actions/runs/39101753234)
and [Windows normal/workspace](https://github.com/17thedevv/luna/actions/runs/39101753210);
[Ubuntu workspace](https://github.com/17thedevv/luna/actions/runs/39101753252)
is queued. Final workspace passes depend on their triage jobs. These are run
locators, not passing evidence.
