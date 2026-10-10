<!-- luna-doc-role: evidence -->

# D1-FU2 report — comptime diagnostic cascade

Branch `d1-fu2-comptime-diagnostic-cascade` (from candidate `5122d3ce`).

## Finding

A single semantic error inside a comptime target produced two diagnostics:

```
const BAD: i32 = comptime { dec value = 1; value.not_a_method() };
fn main() -> i32 { return BAD; }
```

```
error[E1001]: Method `not_a_method` not found for type `Primitive(I32)`
error[E4005]: cannot evaluate comptime block: comptime type mismatch: MVIR invariant
   violated: unresolved method `not_a_method` reached lowering
```

## Verdict: redundant cascade

E4005 is **not** an independent failure. Its message embeds a lowering symptom
(`unresolved method ... reached lowering`) that is a direct consequence of the
already-reported E1001. It adds no new information, so keeping both violates the
diagnostic contract ("a semantic error already diagnosed must not spawn an
additional comptime-failure diagnostic purely from poison/error propagation").

## Root cause (provenance loss at the boundary)

`luna-mvir/src/interp.rs` `finish_preparation` lowers the comptime target to MVIR.
When the generator reports an invariant (`generator.diagnostics` non-empty), the
interpreter flattened the diagnostic **messages** into
`ComptimeError::TypeMismatch("<messages joined>")` — dropping their code and span.
The semantic caller (`report_comptime_error`) then wrapped that generic error into
a **new** `E4005`. The failure's provenance (already-diagnosed vs new) was lost at
the evaluation boundary, exactly the case the task called out.

The mismatch layer is the **comptime evaluation boundary**, not the diagnostic
formatter.

## Fix (provenance-preserving, no "if E1001 exists")

1. `luna-semantic::ComptimeError` gains `AlreadyDiagnosed`.
2. `interp.rs` returns `ComptimeError::AlreadyDiagnosed` when the generator
   reported diagnostics, instead of stringifying them.
3. `typechecker::report_comptime_error` emits **nothing** for `AlreadyDiagnosed`:
   the root cause is already in the diagnostic list.

This distinguishes failures **structurally** (did the failure come from an
already-diagnosed, unlowerable program?not from "does the list contain E1001").
Interpreter-level failures keep their own variants and are unaffected.

## Acceptance matrix

| Case | Result |
|---|---|
| semantic error inside comptime | E1001 only, no E4005 |
| semantic error outside comptime | unchanged (E1001 only) |
| independent comptime failure (`1 / 0`) | E4005 still emitted |
| independent comptime failure (unsupported op) | E4005 still emitted |
| check vs build | identical diagnostic identity |
| source vs fresh artifact | parity |

## Harness / guards

`comptime_diagnostic_cascade_cli` (4) — cascade suppression, independent
preservation, outside-comptime unchanged, check/build identity parity.
Guards: `diagnostic_conformance_cli` 4, `comptime_dependency_precision_cli` 1,
`artifact_execution_dependencies_cli` 1, `adv_comptime_dyn_tests` 16, `ui_tests` 1,
`luna-semantic` 0 failed.

## Workspace

`cargo test --workspace --no-fail-fast` → **221 binaries, 1369 passed / 0 failed /
1 ignored, exit 0**.

Two earlier attempts exited 101 with `Os { code: 112, kind: StorageFull }` at
`support/stdlib.rs:34` — not a regression: `ProviderModes::fresh()` leaves its
`%TEMP%` sysroot behind, and 3467 abandoned `%TEMP%\luna*` dirs (21.3 GB) had
accumulated. Deleting them made the run clean. This is direct evidence for the
R5-HARNESS-01 root cause.

## Versioning

No bump. Diagnostic provenance/presentation only — no comptime execution
semantics, interpreter rules, MVIR, artifact schema or version change
(compiler22 / metadata9 / MVIR5 / format2).

## Verdict

**D1-FU2: CLOSED / FIXED IN TESTED SCOPE.**
