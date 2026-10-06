<!-- luna-doc-role: evidence -->

# C1 reconciliation report — 2026-10-06

Candidate: `codex/luna-0.1-alpha.1-candidate` @ merge `e679d560`.
Base: `3601d12e`. C1 side branch tip: `2615b5f8`.

## Summary

- C1 branch is a linear descendant of `3601d12e`; 5 commits reviewed.
- Merge into candidate auto-resolved (no conflicts); version transition
  compiler 15→19, MVIR 4→5, metadata 7→9 (format 2).
- Canonical sysroot rebuilt at compiler 19: exit 0, 49 providers.
- C1 harnesses all PASS: named_arguments_cli, default_arguments_cli,
  default_omission_cli, comptime_loans_cli.
- Candidate full workspace: **exit 101, 203 targets, 1326 passed / 3 failed /
  1 ignored, 0 StorageFull**.
- Baseline was 197 targets / 1315 / 1 / 1.
- **Delta: +2 new failures** in `luna-driver --test p2a_tests`; A1
  `generic_drop_cli` closure_capture unchanged (expected).
- **Verdict: CANDIDATE NOT ACCEPTED** until the 2 new failures are resolved.

## Review of the 5 commits (C1-R02..R06)

- `1b398714` named-argument binding: parser/relocation, `call_binding`,
  semantic tables, mono/MVIR lowering, canonical parameter names in metadata,
  new `named_arguments_cli` + fixtures. Version 15→16, metadata 7→8.
- `c0818477` comptime root: adds scoped `MonoRoot` (`ctx.comptime_root`, never
  artifact metadata); mono collector on expr/stmt produces concrete call plans
  before early comptime evaluation. Generic; no stdlib-name branch.
- `f36a5abc` comptime admission: driver `CheckedComptimeEngine` runs
  `verify_items_lifetime` + `validate_mvir_module` + interprocedural borrow
  check + redundant-drop elimination before executing prepared comptime; adds
  `ComptimeError::Diagnostics` preserving typed codes/spans.
- `ce417f19` default declarations: AST/parser/relocation for parameter defaults,
  definition-site resolution, canonical default contracts in metadata.
  Version 16→17, metadata 8→9.
- `2615b5f8` default omission: logical callee frame, concrete binder/lifetime
  plans, omission entries; borrowck effect/region updates. Version 17→19,
  MVIR 4→5.

Both comptime commits are generic admission hardening required by C1's early
evaluation; neither is named/default-specific.

## Open finding — `p2a_tests` (C1-R23/C1-R24)

`f36a5abc` routes early comptime through ordinary loan checks, so comptime
rejection now emits canonical typed diagnostics instead of the old bespoke
message strings. Both programs are still rejected correctly (`success == false`);
only the oracle string/code assertions fail.

- `test_case_15_comptime_pointer_escape_rejected`: now emits code
  `LocalBorrowEscape` ("Reference to local variable escapes function scope");
  oracle expects `E_COMPTIME_POINTER_ESCAPE` or "pointer escape".
- `test_case_19_comptime_use_of_moved_value`: now emits code `UseAfterMove`
  ("Use of moved value '%v5'"); oracle expects `E_USE_OF_MOVED_VALUE` or
  lowercase "use of moved value".

C1's only edit to `p2a_tests.rs` was an unrelated +1 line (`lifetime_info` field)
in `test_case_20`; it did not update these two oracles.

Decision needed: accept the unification (update the two oracles to the canonical
typed diagnostics, preserving the rejection requirement) OR preserve
comptime-specific diagnostics (code change) OR maintainer decides. No oracle was
changed in this reconcile.

## Verdict

CANDIDATE NOT ACCEPTED. A1 unchanged; two p2a mismatches must be resolved
before the candidate can be compared as release-candidate-equal-to-baseline.
