# Call arguments and rigid generic fixture gate — 2026-10-05

The adopted [call contract](../../spec/0.1/call-arguments-v1.md) remains required
for 0.1. This checkpoint does not claim named/default argument support or a
release freeze.

## Correcting two invalid method fixtures

The cast repair at e3b4402 correctly rejects `return 0 as T` in an unconstrained
generic body. A call-site choice of `i64` cannot justify a body which promises
to construct every possible `T`. The old `result_inference` and
`qualified_generic` positive fixtures used that invalid operation.

They now call `Construct::create()` under a declared trait bound. Both instantiate
the same method with `i64` and a differently laid-out user-defined `Product`.
The inference test still has no value arguments to infer the return type from;
its expected result type must complete inference. The qualified test preserves
distinct trait `U` and implementation `V` declaration binders. These are genuine
generic constructions, with no name-based binder joining or relaxed cast rule.
`integer_to_rigid_generic.ln` retains `0 as T` as an explicit E2026 rejection.

The permanent method-resolution and cast CLI harnesses both pass against an
immutable copy of the e3b4402 CLI. The method matrix checks source-only and
fresh artifact-only roots, relocated providers and both provider declaration
orders. The cast matrix includes the new rejection in check and build in both
modes. These are focused results: they do not revise an earlier full-workspace
verdict. The harnesses use only Rust std and were compiled directly with
`rustc --test`, using explicit `CARGO_MANIFEST_DIR` and `CARGO_BIN_EXE_luna`
expansion values; this avoids rebuilding a shared CLI consumed by a live test.

Exact fixture/harness hashes, compiler/runtime hashes and lossless logs are in
[the verification report](evidence/method-oracle/exit.json.gz) and its adjacent
evidence files. The original compiler source was not changed by the fixture
repair. Native success is asserted by the permanent harness, not inferred from
type checking alone.

## Named/default baseline is incomplete

The independent [baseline report](evidence/call-arguments-before/observations.json.gz)
records eleven programs against the same e3b4402 CLI:

| Case | Observed behavior | Contract disagreement |
| --- | --- | --- |
| Full positional call | check/build/native succeed | control |
| Reordered historical `b: 2, a: 9` labels | native returns -14 instead of 0 | labels silently ignored |
| Reordered method labels | same wrong native result | method binding also positional |
| Unknown / duplicate labels | accepted | must reject |
| Positional after named | accepted | must reject |
| Structural callable with named labels | accepted and wrong result | must reject without a declaration signature |
| Python `name=expr` labels | rejected before correct binding | parser gap |
| Parameter default expression | rejected | parser/representation gap |
| Missing / excess ordinary arguments | check succeeds, build rejects | semantic arity gap |

Invalid-arity programs were not executed. All probe source and command logs
are retained next to the report, independently of the eventual fixes.

The implementation must preserve explicit argument evaluation order while
mapping values, inferred types and effects to declaration parameter positions.
Trait labels/defaults use the trait signature. Imported public parameter names
must contribute to canonical interface identity. Defaults still require their
full per-call definition-scope, ownership and portable-body contract; accepting
syntax or a literal default is insufficient.

## Workspace evidence limitation

The workspace run started with 1246 tracked inputs pinned to e3b4402. During
the run, the shared checkout moved to an LSP branch and eight pinned files
changed, including Cargo configuration, the CLI entrypoint, SourceManager and
driver files. Its eventual raw result must be preserved with those differences;
it cannot certify an immutable e3b4402 candidate. This work stays in an isolated
checkout based on e3b4402. Closure cleanup, defaults and broader R3–R5 gates
remain open.
