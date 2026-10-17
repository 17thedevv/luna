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
  after returning branches must not create a false E3004.
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

Focused regression and a new immutable implementation pin's complete Windows
and Ubuntu qualification are still required. Earlier pin evidence is historical
only. Candidate advancement and release readiness remain unproven.

SKILL IMPACT: none; existing research, semantic compliance, public-boundary
testing and capability validation instructions already cover this defect.
