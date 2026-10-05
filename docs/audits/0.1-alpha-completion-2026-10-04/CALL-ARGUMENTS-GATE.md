# Call arguments and rigid generic fixture gate — 2026-10-05

The adopted [call contract](../../spec/0.1/call-arguments-v1.md) remains required
for 0.1. The dated baseline and positional repair below are followed by the
named-binding checkpoint. Defaults and generic comptime remain incomplete;
this document does not establish a release freeze.

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

### Implementation sequence after the arity prerequisite

This is an implementation plan under the adopted contract, not a new language
policy or a claim that the following stages are implemented.

1. Introduce one semantic binding plan which maps each explicit source argument
   to a declaration parameter ordinal. Keep the AST argument list in source
   order. Candidate applicability, expected argument types and bounds consume
   that mapping. Validate unknown/duplicate labels and positional-after-named
   before selecting a method; use the trait declaration's signature for trait
   calls. Structural callable values keep full positional arity.
2. Carry the plan through provider extraction/injection, concrete mono calls
   and MVIR lowering. Lower explicit expressions once in source order, then
   pass values in parameter order. Test effect order and moves of user-owned
   values: delayed operand loads must not let a later argument change an
   earlier argument's value. Borrow and return effects use parameter ordinals.
3. Include public parameter names and declaration capabilities in the canonical
   callable interface. Keep structural function-type identity distinct from
   declaration labels. Update schema/compatibility epochs when representation
   changes, rebuild artifacts officially, and verify stale-dependent rejection
   plus unchanged-body and binder-renaming controls.
4. Add parameter default expressions to AST, relocation and lexical resolution.
   Resolve in the defining scope with only earlier parameters available. Check
   defaults under rigid generic binders and ordinary ownership/unsafe rules.
   Cover invalid receiver defaults, required-after-default and later-parameter
   references. Reconcile formal grammar and examples with the adopted syntax.
5. Materialize only needed omission patterns while preserving a logical callee
   parameter frame. Initialize provided parameters, evaluate omitted defaults
   in declaration order, then execute the original body with its normal
   move/drop/borrow machinery. The full-arity ABI remains unchanged; omission
   entry identity includes its pattern and concrete generic substitution.
   Avoid exponential eager specialization or zero/null stand-ins for absent
   arguments. A forwarding wrapper alone is insufficient if moving a supplied
   owner into another frame invalidates a default borrowing that owner.
6. Preserve definition-site helpers, parameter/generic identities and default
   bodies across relocated artifact-only providers. Map trait parameter roles
   to implementation roles by ordinal, and derive actual omission-entry effects
   rather than blindly copying full-arity lifetime parameter indices. Verify
   private-helper hygiene, earlier-parameter borrowing, fresh owned defaults,
   omitted/overridden side effects, move/escape negatives and public freshness.
7. Run the full call-contract CLI matrix, focused regressions and a new immutable
   workspace candidate. Record named and default dimensions separately until
   both satisfy the complete contract; neither a parser pass nor this positional
   arity repair closes CALL-ARGUMENTS-v1.

## Positional arity repair and unsized diagnostic compatibility

Ordinary function calls previously skipped semantic arity checks unless they
were closures or memory intrinsics. The candidate validates every callable's
required positional arity before lowering. Implicit receiver identity comes
from the method declaration instead of being guessed from the supplied argument
count. A structural function-pointer field remains an ordinary callable.
Explicitly variadic declarations retain their fixed-prefix minimum at semantic
checking; this does not claim native variadic ABI support.

The initial arity candidate passes 195 semantic tests, the method and cast
matrices, and the new arity CLI matrix: 52 typed check/build rejections plus two
native controls in fresh source/artifact modes. A check-only variadic control
passes in both modes. The memory/callable suite passes 3/3 through Cargo.
[Its exact pin and commands](evidence/call-arity/verification-exit.json.gz) are
retained along with [the Cargo memory retry](evidence/call-arity/memory-cargo-exit.json.gz).
The initial verification script incorrectly tried to compile that dependency-
using memory harness with bare rustc; its failed compile/log remain preserved.
No compiler failure is inferred from that missing-harness-dependency error.

The completed workspace also exposed a compatibility regression in cast
diagnostics: rejecting a cast to an unsized value lost the existing
`E_UNSIZED_TYPE_IN_VALUE_POSITION` category and TypeMismatch code. The amended
candidate restores that classification using the semantic target type while
keeping centralized cast rejection and poison propagation. The original driver
oracle is unchanged; all 14 adversarial raw/dyn driver tests now pass. A new
public `unsized_value.ln` rejection is included in the cast CLI suite. The
[amended pin](evidence/call-final/start.json.gz) separates this diagnostic change
from the earlier arity evidence; final cast/span/char results are recorded with
that candidate, not retroactively assigned to the initial one.

The amended candidate builds its canonical sysroot and passes all three CLI
cast, UTF-8/tab span and char suites, with inputs/runtime unchanged. The
[terminal report](evidence/call-final/exit.json.gz) records its new binary hash,
the 14 driver and three CLI passes, and exact command/log hashes.

Compiler/library boundary: this is generic callable validation and unsized
diagnostic handling, benefiting arbitrary user functions/types. No library
name/layout, intrinsic, lang item, artifact schema or native ABI is added.
SKILL IMPACT: a minimal testing-strategy refinement requires check/build arity
agreement and preserving the original purpose when repairing generic fixtures.

## Workspace evidence limitation

The workspace run started with 1246 tracked inputs pinned to e3b4402. During
the run, the shared checkout moved to an LSP branch and eight pinned files
changed, including Cargo configuration, the CLI entrypoint, SourceManager and
driver files. Its raw result is preserved with those differences. The run ends
exit101 with 1311 passed, three reported failed, one ignored and **four failed
targets**. The generic-drop target exits abnormally without a test-result
summary, so the reported test counts do not count every failed target. Span
acceptance fails during its fresh sysroot child process (0xffffffff);
generic-drop's harness also ends 0xffffffff. Their underlying causes are not
established by that log. Method resolution exposes the two invalid generic
fixtures repaired above; the driver raw/dyn suite exposes the unsized diagnostic
classification loss. Neither focused fixes nor unexplained process exits
rewrite the raw verdict.

[Start pin](evidence/workspace-cast-start.json.gz),
[terminal report](evidence/workspace-cast-exit.json.gz) and
[complete log](evidence/workspace-cast.txt.gz) retain that run. It cannot certify
an immutable e3b4402 candidate. This work stays in an isolated checkout based on
e3b4402. No shared LSP branch changes are merged or overwritten.

The immutable Ubuntu e3b4402 platform workflow also fails: its workspace command
is fail-fast and stops on `closure_capture`, native exit3 in both modes.
[Complete job log](evidence/call-arity/ci-e3b-job-111721303256.json.gz) and
[job statuses](evidence/call-arity/ci-e3b-jobs.json.gz) are retained. Runtime ABI
on Ubuntu/macOS, Ubuntu ASan and whole-file I/O on both platforms pass within
their job scopes. The 1a9ea27 workflows were still running at the recorded
snapshot; no later verdict is inferred. Closure cleanup, defaults and broader
R3–R5 gates remain open.

## Named-binding implementation checkpoint — 2026-10-05

**CALL-ARGUMENTS-v1: PARTIAL.** Direct declarations now bind both `name=value`
and historical `name: value` arguments to parameter ordinals. Checking uses the
bound parameter's type for inference; evaluation still follows source order.
Each value is captured/transferred when evaluated, before call operands are
permuted. This preserves scalar snapshots, reads before ownership moves and
cleanup when a later argument returns early. Invalid labels, duplicate binding,
missing/excess arguments and positional-after-named calls reject with E2001.
Structural function values reject named calls even when an immutable target is
known. Trait declaration labels govern calls to implementations whose private
parameter spellings differ. Receiver, unsafe, generic, lifetime and move rules
remain independent of label ordering.

Portable callable signatures carry public labels separately from structural
function type and lifetime/provenance identity. Checked call plans contain
ordinals, not session-local IDs. Exported functions, trait methods and impl
contracts preserve signatures; the independent decoder validates/reconstructs
them. Public parameter renaming changes the interface hash and rejects stale
dependents. Impl parameter spelling, generic binder renaming and a body-only
edit preserve the tested interface. Compiler protocol is now **16**, semantic
metadata **8** (file schema 2, MVIR 4 unchanged). The canonical isolated sysroot
was rebuilt officially: all 49 providers succeeded.

Comptime argument temporaries receive ordinary cleanup scopes and reachable
typed drop glue. References retain the same VM handle across reads; a required
destructor sees live storage before its slot becomes moved. User-defined owned
values establish one-drop behavior; moved values and escaping local references
still reject. No missing callee or required destructor becomes a no-op.

### Evidence and limits

[Terminal report](evidence/named-arguments/exit.json) preserves exact commands,
input/runtime/compiler hashes and all raw logs, including earlier failed
harness/API/setup attempts. The broad candidate is pinned separately from the
amended candidate; broad results are not retrospectively assigned to a new
binary. After broad verification, two UTF-8 comments were restored and scalar
snapshot/return-loan controls plus a borrow negative were added. The amended
pin was observed during its run, not claimed as a pre-command pin; all 887
recorded inputs, binary and runtime agree at the final observation.

- The final named CLI harness passes in independent fresh source-only and
  artifact-only roots, with the user provider source removed and the project
  relocated. It observes 56 typed `check`/`build` rejections, ten positive native
  executions, a fresh dependent native control, four interface controls and
  stale-dependent rejection in both commands.
- The broad internal run passes 324 tests across semantic, metadata, MVIR,
  borrowck and parser crates. A separate run passes 41 existing driver/comptime/
  lifetime/interface tests. Their complete terminal Cargo summaries are retained;
  numeric shell exit statuses were unavailable after tool handles were lost.
- The amended named harness and seven existing comptime execution-availability
  invariants pass with observed shell exit0. This includes missing-body and
  required-destructor rejection, already-dropped/uninitialized controls and
  forbidden effectful/extern destructors.
- The broad seven-target CLI command remains **FAIL: eight tests passed, one
  failed**. `generic_drop_cli` reports `closure_capture` native exit3 in both
  modes. All raw failure evidence is retained; this known independent baseline
  issue is not closed by named-call success.
- Both named and positional generic comptime reproducers fail source-only
  `check` and `build`: E4005 cannot find concrete callee `_MFN5named7consumeE`.
  These four probes establish a generic early-instantiation gap; they are
  required positive cases, not intentionally unsupported or waived behavior.
- Parameter defaults still lack AST/evaluation/portable implementation. The
  grammar now records adopted syntax, which is not implementation evidence.
  Omission-entry ownership, definition-site hygiene, earlier-parameter borrows,
  generic/trait defaults and source/artifact execution remain required.

The impl/method binder decision remains independent: matching names never join
two generic binders. Bound-sensitive stdlib methods use separate bounded impls.
Named declaration labels do not alter that rule.

```text
Compiler Change
    Capability: Declaration argument binding, ordered value capture, portable
                callable labels and generic owned temporary cleanup at comptime.
    Why stdlib exposed it: Adopted calls and owned values require correct ordinary
                         invocation; no library API supplies special lowering.
    Why it is generic: Operates on callable/parameter/type identities and existing
                       ownership/drop primitives, for arbitrary declarations.
    User-defined type benefiting: named::Owned and named::Counter in CLI fixtures.
    Tests: named_arguments_cli, call_argument_parser_tests, semantic/metadata
           invariants, comptime_execution_availability and scoped regressions.
    New intrinsic/lang_item?: NO; full-arity native ABI unchanged.
    Stdlib-specific branch?: NO
```

SKILL IMPACT: **REFINEMENT** of mellis-grammar (adopted named/default surface with
an explicit implementation gate) and luna-semantic-compliance (public callable
labels are distinct from ordinal lifetime/provenance identity). Both changed
skills were reread in full and checked for conflicting guidance. No release
authority, name-based generic identity or new intrinsic is introduced.

Next: repair concrete generic callee availability at the early comptime root,
then complete default expressions through the logical callee parameter frame.
Closure policy awaits maintainer input; R3–R5 and the full retained contract
map remain required. This checkpoint does not close the overarching goal.
