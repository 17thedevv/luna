# R5 follow-up — reachable missing return (E3004)

Date: 2026-10-17. Status: REPAIR VERIFIED IN FOCUSED SCOPE;
full exact-pin qualification remains IN PROGRESS.

The retained [diagnostic contract](../../diagnostics/diagnostics-v1.md) assigns
E3004 (`MissingReturnValue`) to Borrowck for a non-void function reaching its
end without returning. This repair enforces that existing rule; it introduces
no syntax, ABI, artifact layout or language policy change.

## Research and semantic preflight

- Syntax: `fn f() -> i32 { }`, plus ordinary calls, generic instantiation,
  methods, closures, defaults and async bodies under the versioned syntax.
- Invariant: a reachable fallthrough cannot manufacture an initialized result.
  Void fallthrough and explicit/tail returns remain valid. Unreachable joins
  after returning branches must not create a false E3004. Literal/materialized
  boolean branches have only their executable successor: an endless loop has
  no fallthrough, while a reachable `break` makes its exit subject to E3004.
- Owning path: AST/semantic checking → monomorphized MVIR generation → reachable
  CFG return analysis in Borrowck → diagnostic rendering. Invalid paths stop
  before native emission. Imported generic bodies follow this same path.
- Root cause: `generate_fn_body` converted non-void fallthrough into
  `Ret(Number("0"))`; the existing `MissingReturn` terminator was never produced.
- Repair: emit the existing terminator; preserve a real enclosing declaration
  source anchor through existing MVIR instruction debug data, including empty
  bodies, empty joins, closures and generated default entries. Async admission
  considers the semantic output type rather than the future ABI wrapper.
- Source/artifact obligation: build a fresh provider, remove its source in
  artifact mode, then require imported generic fallthrough to reject E3004;
  valid imported functions must still execute correctly.
- Foreign assumption avoided: backend zero/unreachable is not a source-level
  default return value or evidence of definite initialization.

## Evidence and remaining work

At baseline `5d20e57156038a4b43c11734a9b4bd5ce58cd206`, the permanent
`tests/luna/language/missing_return/empty.ln` reducer exits 0 under `luna check`.
Raw characterization: [baseline log archive](evidence/baseline-check.log.gz).

The permanent CLI suite is `missing_return_cli`. Its matrix covers empty,
partial-branch, loop-exit, generic, method, closure, async and default-entry
bodies, plus imported generics, in source-only and fresh artifact-only roots.
Each negative must fail `check` and `build` with spanned E3004, no E6001 and no
published executable. Positive controls must check, build and run with exit 0.
The [focused run](evidence/missing-return-cli.log.gz) passed: 36 negative command
checks and both modes' valid check/build/native execution; Cargo reports
1 passed, 0 failed. Inventory regeneration and drift checking also pass;
E3004 now has behavioural coverage and is no longer declared unreachable.
The MVIR/Borrowck internal regression also passes: 73 passed, 0 failed;
raw logs and SHA-256 hashes are preserved in `evidence/`.

The [expanded focused run](evidence/missing-return-cli2.log.gz) also passes:
40 negative command checks, plus source/artifact native controls containing
endless `while true`, conditionless `for`, and a constant-true return branch.
The endless functions are compiled but not invoked. A reachable `break` is a
separate E3004 negative. MVIR/Borrowck regression was rerun after the
reachability change: again 73 passed, 0 failed.

The initial implementation pin `e5f234a9` was superseded after an additional
control exposed a false E3004 for `while true { continue; }`. Windows acquisition
at `D:/r5-e5f234a9/windows` was explicitly cancelled during CLI build, after
runtime ABI passed; incomplete build evidence is preserved, not reused.
[CI run 18](https://github.com/17thedevv/luna/actions/runs/39031641783) tests that
superseded pin and cannot qualify the subsequent reachability repair.

Focused regression and a new immutable implementation pin's complete Windows
and Ubuntu qualification are still required. Earlier pin evidence is historical
only. Candidate advancement and release readiness remain unproven.

SKILL IMPACT: none; existing research, semantic compliance, public-boundary
testing and capability validation instructions already cover this defect.
