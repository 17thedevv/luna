# Replacement qualification requirements

The original [R5 checklist](../0.1-alpha-completion-2026-10-04/r4/r5-checklist.md)
and [full completion matrix](../r5-linux-native-link-2026-10-18/REQUIREMENTS.md)
retain their scope. This replacement must reacquire all eight lanes after the
move-origin and diagnostic identity/order repairs; no historical PASS substitutes for a new-pin result.

| Required evidence | Current status |
| --- | --- |
| Ubuntu normal: native build/runtime/49 providers, focused/diagnostics, target, docs and quickstart | PENDING NEW PIN |
| Ubuntu sanitized C runtime ABI and partial-read cleanup | PENDING NEW PIN |
| Ubuntu comptime/VM/layout boundary, with characterizations separate | PENDING NEW PIN |
| Ubuntu full workspace no-fail-fast triage | PENDING NEW PIN |
| Ubuntu full workspace default-parallel final after triage | PENDING NEW PIN |
| Windows native normal: build/runtime/49 providers, 12 focused batches, diagnostics, docs, quickstart, comptime and strict L002 replay | PENDING NEW PIN |
| Windows full workspace no-fail-fast triage | PENDING NEW PIN |
| Windows full workspace default-parallel final after triage | PENDING NEW PIN |
| Diagnostic provenance | New CLI reducers for nested/equal-state/loop joins, field reinitialization, compacted fields and imported generic artifacts; exact primary/related lines, repeated checks, build parity and native valid controls |
| Diagnostic identity/order | Distinct identical-wording E3002 events; source order across multiple functions and independent check processes; check/build and source/artifact parity; structural-cause union and idempotence |
| Internal dataflow | Predecessor permutations, origin-only convergence, compacted field provenance and subtree invalidation; affected borrowck regressions |
| DIAG-1..10 final review | Preserve typed identities, primary/structural related spans, poison, deterministic identity, phase admission and the 72-code inventory; eight reserved and four non-portable fixture limits stay explicit |
| Final closure | Verify every archive/hash/exit/target/mode, reconcile public docs and ledger, then fast-forward candidate only after all requirements pass |
| Maintainer decision | Tag, main merge, PR creation and publication remain reserved; deliver a concrete qualified handoff first |

Previous implementation `3a72f946` completed all eight lanes, including four
workspace passes of 1414/0/1, but failed final diagnostic-origin review. Its
complete [evidence](../r5-linux-native-link-2026-10-18/README.md) is historical.
Candidate remains `c085449b` until the replacement qualifies.

## Integration closure (2026-10-18)

Two repair lines existed and were integrated before qualification, so the eight
lanes below run once at a single implementation SHA:

| Line | Content | Status |
| --- | --- | --- |
| `r5-diag-repair` (base `5d20e571`) | DIAG-1..10 repair wave, canonical target selection (`canonical_supported_target`), typed comptime `E4002`/`E4003`, `run` no longer reclassifies compiler diagnostics as `E6009`, per-code coverage suite | verified end-to-end at `c085449b` (candidate) |
| `r5-missing-return` (`ce886d09`) | duplicate-definition/candidate handling, language-item and coherence ownership, symbol/registry identity, semantic-table and metadata anchors, declaration location CLI suite + fixtures | local preflight only (reused runtime, no fresh clone) |

Integration is the merge commit `8525067f` on `r5-diag-repair` (clean merge, no
conflicts). Verified on the merged tree: `cargo check` clean;
`diagnostic_declaration_locations_cli` 3/3; per-code coverage suite;
`luna-common` 15 (registry parity + per-code identity/span policy);
`luna-semantic` 27.

**Retraction (E3004).** The DIAG-7 coverage work recorded "E3004 has no
producer: a non-void body that can fall through is accepted". That was an
artifact of the divergent base, not a compiler defect: a probe at the integrated
tree reports `E3004` for both `fn f() -> i32 { }` and a declaration-only body,
`missing_return_cli.rs` covers it, and the producer inventory now records
`E3004` as produced with an anchored producer
(`luna-borrowck/src/return_analysis.rs`) plus automated coverage. The corrected
inventory is produced 64 / reserved 8 / unreachable-in-test 4 / coverage debt 0,
where the four declared limits are `E3007`, `E6002`, `E6003`, `E6009` with their
reasons.

**Diagnostic provenance and identity/order** requirements are closed by the
integrated suites: declaration locations (`E1002`, `E2001`, `E2005`, `E2006`,
`E2019` with structural related labels and repeated-check idempotence), the
comptime codes, the per-code identity/span-policy contract test and the 72-code
inventory that the diagnostics lane re-verifies.

## Lane acquisition at the integrated pin

| Lane | Where | Evidence |
| --- | --- | --- |
| Windows native normal | local exact-pin runner | `D:/r5-8525067f/windows/events.jsonl` |
| Windows workspace triage / final | local exact-pin runner | same runner, `workspace-*` labels |
| Ubuntu normal / runtime-asan / comptime-vm / workspace triage / workspace final | CI workflow run 23 (`aa3ed8ab` → tested `8525067f`) | artifacts `r5-ubuntu-*` |

The focused diagnostics batch in every lane now includes
`diagnostic_declaration_locations_cli`, `missing_return_cli` and
`diag_per_code_coverage_cli`, so identity, spans and declaration coverage are
gated together at the pin.

No PASS is carried from any earlier pin; the candidate moves only after all
eight lanes pass at `8525067f`.
