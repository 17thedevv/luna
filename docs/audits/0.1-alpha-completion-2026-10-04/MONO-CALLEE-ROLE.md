# Direct callees and function values — 2026-10-05

This is a repair checkpoint in the active R0–R5 plan, not a release verdict.
Base revision: `f832db24459b5a2675dec1c6836f3bc1502340a9`.

## Failure and first incorrect representation

The completed [workspace before repair](evidence/workspace-raw-slice.txt.gz)
failed five targets with E5001 at the generic barrier: method resolution,
interface constraints, canonical global namespace execution, phase4c parity,
and standard conversions. The source qualified-trait fixtures reproduce the
failure independently of standard-library names.

Commit f832db2 added a fallback empty substitution when an identifier denotes
an AST function without its own generic parameters. The visitor also traversed
a direct call's callee in the function-value role. An abstract trait method
with implicit Self consequently entered the worklist with an empty substitution,
even while the surrounding call selected a concrete checked implementation.
No own method generics does not mean no ambient trait/impl binders. The barrier
correctly rejected that spurious abstract instance; weakening it would hide the
incorrect worklist entry.

## Contract and repair

Concrete calls preserve declaration/impl/method binder identities. Generic
function values retain their checked substitutions and callable safety. Impl
and method parameters remain independent even if their spellings match.

The mono visitor now records an immediate direct-callee role only for the
callee node. Its surrounding call selects/enqueues the actual instance. Nested
arguments, returned callbacks and other value expressions keep ordinary
function-value discovery. The role is a recursive visit argument, not a global
ExprId exclusion set or a flag inherited into nested arguments. The generic
barrier and trait ambiguity/bound rules are unchanged.

The permanent `nested_callable_role.ln` combines a qualified trait call with an
explicit generic function value, a callback argument, a returned callback and
a nested direct generic call. The method matrix executes it in both provider
modes and both declaration orders. Existing memory/callable fixtures retain
positive/negative safety and identity controls.

The separate STRUCT-LIFE-17 oracle is updated from metadata5 to metadata7,
which carries portable call effects with both direct and carried return sources.
Its exact-version and incompatible-header assertions stay active; reader tests
continue to reject old protocol/schema payloads before decoding.

## Verification

The [candidate pin](evidence/mono-role-candidate-pin.json) records the source
snapshot, CLI/runtime hashes and command exits. The fresh 49-provider sysroot,
five CLI harness tests, 43 driver tests and 211 semantic/metadata tests all pass.
The [expanded view matrix](evidence/mono-role-enum-views.txt.gz) passes one
harness with 14 native and 72 typed-rejection observations across source and
artifact modes, including generic enum wrap/unwrap and borrow-death controls.
No expected ambiguity, bound, move, escape or unsafe rejection is removed.

These results do not turn the earlier workspace FAIL into PASS. A new complete
workspace run is required on this repaired candidate. Owned closure cleanup,
named/default arguments and broader R3–R5 completion gates remain open.

## Compiler boundary and skill impact

Capability: correctly distinguish direct call instantiation from function-value
instantiation. Stdlib exposed the generic defect through conversions. The same
fix benefits arbitrary user-defined traits and callbacks, including Read/Counter
in the permanent CLI fixture. No intrinsic, lang item, stdlib-specific branch,
source-name mapping or new syntax is introduced. Source/artifact loader policy
and independent generic binders remain unchanged.

SKILL IMPACT for this mono repair: none. Existing capability-validation guidance
already requires callback-value, substitution, source/artifact and native checks.
The accompanying raw-view work separately refines the capability/semantic skills.
