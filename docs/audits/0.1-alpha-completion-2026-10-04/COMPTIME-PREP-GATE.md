# Early comptime preparation and loan admission — 2026-10-05

This gate follows the [named binding checkpoint](CALL-ARGUMENTS-GATE.md).
CALL-ARGUMENTS-v1 and the full 0.1 goal remain required. Generic callee
availability and safe loan admission are separate obligations.

## Generic callee root cause and repair

The existing `MonoCollector::run_on_expr`/`run_on_stmt` APIs were not called at
early evaluation. They also discarded the root's checked call plans before
processing reachable functions. Consequently the VM received an empty-
substitution symbol for a generic call, although ordinary semantic checking
had recorded its concrete substitution. Named and positional owned calls both
failed E4005; their original source-only logs remain in the named checkpoint.

The APIs now return `MonoRoot`: concrete expression/symbol/pattern types,
call instances, try protocol and foreach plans. A root has no fabricated
function declaration identity. Semantic analysis prepares only reachable
instances before calling the engine and temporarily supplies them with the
root. It restores prior compilation units/root afterward, while interned
types stay valid in the same session. The ordinary whole-program pass owns
the final compilation units. MVIR consumes these plans; it does not perform
type checking or guess generic substitutions. Required concrete callees and
destructors retain fail-closed lookup and concretization barriers.

Permanent CLI cases exercise imported nested generic calls, multiple concrete
types, method and qualified trait generics, reference returns, local generic
consumption and a generic destructor declared after the constant. Generic
move/escape/effect negatives retain their original semantic purpose. The
provider is built freshly, its source removed and its graph relocated for
artifact-only execution. This is a scoped generic availability repair, not
proof of all compile-time ownership or language conformance.

The [terminal record](evidence/comptime-root/exit.json) pins 765 compiler,
library and fixture inputs before the final commands. All remain unchanged;
compiler/runtime hashes match at completion. The internal command passes
324 tests and the separate CLI/driver command passes 45, both shell exit0.
The expanded named harness observes 68 typed check/build rejections and 18
positive native executions across fresh source/artifact modes, plus the
interface and dependent freshness controls. Raw build/development logs, final
commands, hashes and exits are retained separately; the earlier failed full
workspace and closure verdicts are not rewritten.

Compiler Change: generic early root specialization and reachable dependency
availability benefit arbitrary functions/traits and user-defined Owner<T>.
It uses existing semantic instances and MVIR/drop primitives; no stdlib name,
layout, intrinsic, lang item, native ABI or portable schema is added. Protocol16
and metadata8 remain unchanged. SKILL IMPACT: none beyond the named checkpoint's
two refinements; the stage ownership and generic-barrier guidance already
requires this behavior.

## New confirmed loan admission defect

An independent comparison uses the same function accepting two `&rw i32`
parameters and passes the same local storage to both. Native `check` rejects
E3003. Inside `comptime`, `check` and `build` succeed and the resulting native
control exits0. The VM has executed a program that violates the safe loan
contract. It runs before the driver's ordinary borrow stage; materializing
the evaluated constant means that later analysis cannot recover the invalid
original loans. This is a correctness gap, not a privileged comptime rule.
The probe proves compile/runtime acceptance on this candidate; it does not
attribute introduction to the current generic repair or assert native heap
corruption.

Next repair:

1. Split MVIR evaluation preparation from execution. Preparation returns the
   exact checked root and reachable module, retaining spans and identities.
2. Have the driver orchestrate the existing lifetime/borrow checks over that
   program before VM execution. Keep borrowed analysis in its owning crate;
   do not introduce a circular MVIR-to-borrowck dependency or compiler-local
   alias rules that differ from ordinary execution.
3. Compute call summaries for the same reachable program and apply ordinary
   drop-flag cleanup. Execution must consume that verified program, not lower
   a second unchecked copy.
4. Preserve original typed diagnostics and primary spans. Test aliased mutable
   arguments, conflicting shared/mutable use, move/drop, local escape and
   returned-loan transport, with legal repeated reads and sequential mutation
   controls in source/artifact modes. Include imported bodies and all driver
   entry paths that configure a comptime engine.
5. Keep VM resource/effect/escape checks as separate enforcement. Their direct
   invariant tests still matter; successful borrow checking does not permit
   extern side effects or heap/reference escape.

After that, defaults still require definition-site resolution, rigid generic
checking, omission entries sharing a logical callee frame, portable bodies and
full ownership/effects acceptance. A literal-only implementation or forwarding
wrapper cannot replace that adopted contract. Closure capture policy and
R3–R5 remain open; no merge, tag or release verdict follows from this gate.

## Follow-up: ordinary admission before execution — 2026-10-05

The confirmed alias counterexample above is now rejected before VM execution.
MVIR preparation returns `PreparedComptime`, retaining the root and reachable
concrete module. The driver's checked engine performs ordinary declaration
lifetime verification, pre-borrow MVIR validation, interprocedural summaries,
borrow/drop-flag checking and redundant-drop cleanup, then post-borrow
validation. The VM executes that same verified program. All five production
engine construction paths use this orchestration; the low-level interpreter
engine remains available to direct VM invariant tests. MVIR does not depend on
borrowck or implement separate loan rules.

Owning-phase diagnostics cross the comptime API without changing their typed
codes or primary source spans. Invalid move cases now produce E3001, local
escape E3005 and loan conflicts E3003; VM effects continue to produce E4005.
Poisoned constant initializers do not execute again during later typechecking.
This changes the diagnostic oracle to the earlier owning phase while retaining
the invalid programs and required rejections. VM effect, resource and address
escape guards remain independently enforced.

The new CLI matrix checks local/imported mutable aliases, shared/mutable
conflicts and transported return loans: 20 check/build rejections, source
snippet/caret evidence and no emitted executable across fresh source/artifact
modes. Two native controls retain sequential mutation, repeated shared reads
and precise selected-parameter return provenance. The imported provider is
published through the CLI, its source removed and its project relocated.
The named matrix retains 68 check/build negatives and 18 native positives,
including generic/drop cases. Module-const storage and pointer-escape
regressions also pass in both modes; module storage lifetime is unchanged.

The [terminal record](evidence/comptime-admission/exit.json) pins 774 compiler,
library and fixture inputs before the final commands. Every input and both
compiler/runtime hashes agree at completion. `cargo check --workspace
--all-targets` exits0; the internal regression passes 324 and the separate
CLI/driver regression passes 43, both exit0. Lossless raw logs retain the first
development failure (obsolete E4005/string-only oracles), corrected 25-test
development pass, source probes and final commands/exits. An initial scratch
pin setup failed before Cargo ran; the corrected pre-command pin is used.
Earlier committed evidence and failed workspace/closure verdicts are preserved.

```text
Compiler Change
    Capability: Ordinary move/loan/escape admission before early comptime.
    Why stdlib exposed it: Generic and owned constant evaluation needs the same
                         safe language rules as ordinary user execution.
    Why it is generic: Existing lifetime, MVIR, interprocedural and borrow/drop
                       analyses operate on arbitrary checked callable bodies.
    User-defined type benefiting: Owner<T> and imported user functions in the
                                 generic/drop and comptime loan CLI controls.
    Tests: comptime_loans_cli, named_arguments_cli, module_const_storage_cli,
           adv_comptime_dyn_tests, struct lifetime and VM invariants.
    New intrinsic/lang_item?: NO; no native ABI or portable epoch change.
    Stdlib-specific branch?: NO
```

SKILL IMPACT: **NEW DURABLE RULE** in luna-language-capability-validation:
admit the original prepared program before VM execution and execute the same
verified IR; materialized constants cannot retrospectively validate loans.
The changed skill was reread in full and checked against related testing,
semantic compliance, phase ownership and comptime guidance without conflict.

This closes the recorded loan-admission counterexample within the tested scope.
CALL-ARGUMENTS-v1 remains **PARTIAL** because default expressions are not yet
implemented. Known owned closure_capture native exit3 in both modes was not
rerun or closed here. Full R0–R5, pending closure/overflow/target decisions and
remaining release conformance stay required; no merge, tag or release follows.
