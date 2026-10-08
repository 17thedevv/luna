<!-- luna-doc-role: evidence -->

# C4 report — comptime execution-dependency precision

Branch `c4-comptime-dependency` (from candidate `bf265798`).

## Method

Freeze matrix first (no compiler change). The invariant under test is
`interface fingerprint != execution fingerprint`, plus: a comptime result depends
on exactly the providers it executed.

## Reducers (`luna-rs/crates/luna-cli/tests/comptime_dependency_precision_cli.rs`)

| Reducer | Situation | Result |
|---|---|---|
| C4-1 | direct execution dependency body changes (interface unchanged) | consumer **rejected** with a typed stale diagnostic |
| C4-2 | transitive leaf body changes | consumer **rejected** |
| C4-3 | an imported provider that is never executed changes | consumer **accepted** — no false-positive invalidation |
| C4-5 | same interface, changed comptime body | covered by C4-1 |
| C4-6 | source vs fresh artifact | both build and run (parity) |
| C4-7 | stale rejection diagnostic | `dependency execution fingerprint mismatch for provider ...` |

`artifact_execution_dependencies_cli` (native relink vs materialized
invalidation) remains green.

## Precision limitation (recorded, non-blocking)

**C4-FU1 — branch-sensitive dependency selection. Severity: SOUND BUT
IMPRECISE (over-approximation), not a missing dependency.** The comptime
dependency set is a static call-graph walk from the evaluated function
(`luna-mvir::generate_comptime_dependencies`), so a provider reachable only
through an untaken branch can still be recorded. A `comptime { if false { A }
else { B } }` probe returned `0` rather than the else-branch value, so
comptime if-expression semantics were not confirmed and no reducer was shipped
(the probe was withdrawn rather than kept with a wrong expectation). No reducer
produced an **unsound missing** invalidation.

## Result

- `comptime_dependency_precision_cli`: 1 passed / 0 failed.
- Full workspace: 213 binaries, 1345 passed / 0 failed / 1 ignored, exit 0.

## Versioning

Test-only: no serialized execution-dependency payload or codegen change, so the
protocol stays **compiler22 / metadata9 / MVIR5 / format2**.

## Verdict

**C4: CONFORMANT IN TESTED SCOPE** — direct, transitive and unused-import
dependency precision, source/artifact parity and typed stale rejection all hold;
the over-approximation for untaken branches is filed as non-blocking C4-FU1.
