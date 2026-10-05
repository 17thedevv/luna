# Execution ledger — Luna 0.1 completion

Baseline implementation: `c8d0559500e78ef85c96ac842e824e639917cbad`.
This supplements the dated [plan](README.md); no full release PASS is claimed.
Current state remains **PARTIAL**. Revision-specific checkpoints are recorded
chronologically below. The last completed full workspace run remains FAIL at
`39a2a9b`: **1,262 passed, 34 failed, 1 ignored**, exit 101. Later focused
results do not change that immutable verdict.

Earlier artifact-compatibility checkpoint: `f5936c3344788168320dfd4b661532c2cfad8a8d`
(artifact compatibility), following the async/closure repair `363eaf5`.
Field-cleanup baseline: `3d6916673f1ef16a1356f5fe592a13db96efa29c`.
The field-cleanup checkpoint below records its bounded evidence and the
remaining closure failure. Its completed full workspace run exits 101:
**1,276 passed, 6 failed, 1 ignored**. Later focused repairs below are separate
evidence; they do not change that immutable full-run result.

Earlier implementation checkpoint: `7f818610428da3b39565f2f8a88386b0dfc4707c`
(`fix(compiler): validate trait bounds and reconcile provider artifact identities`).
The documentation checkpoint preserves intermediate failures and the later
specific passing reruns; it does not recast those failures as a full-workspace
PASS. Full workspace tests, native target gates and clean-checkout verification
remain R5 work.

## Maintainer decisions and added scope — 2026-10-04

- Option A adopted: [METHOD-RESOLUTION-v1](../../spec/0.1/method-resolution-v1.md).
- Added named arguments and default values to 0.1; defaults evaluate on each
  omitted invocation, in the defining scope:
  [CALL-ARGUMENTS-v1](../../spec/0.1/call-arguments-v1.md).
- These additions remain implementation/acceptance tasks. The completion goal
  includes them as well as the original R0–R5 plan. Runtime overflow and final
  advertised target matrix are still explicit decisions, not silently deferred.
- Module-level constants have immutable program-lifetime storage when borrowed;
  local constants retain lexical lifetime:
  [MODULE-CONST-STORAGE-v1](../../spec/0.1/module-const-storage-v1.md).
  The maintainer adopted this decision after the compatibility checkpoint.

## Evidence and current findings

| Item | Evidence | Current state |
|---|---|---|
| R0 stale oracles/layout | [first regression run](evidence/r0-regressions.txt) | 7 targeted suites; 55 passed, 2 failed. Whole-File I/O 3/3, path 2/2, combinators 9/9, parity 5/5 and numerics 6/6 passed. Struct lifetime 23/24; remaining message-reason assertion subsequently corrected but not yet rerun |
| R0 HT7 isolation | Same run | No canonical artifact overwrite; isolated source/artifact run exposes __raw_table interface fingerprint mismatch. 7/8 storage cases pass; parity remains open |
| R1 generic drop triage | [initial probes](evidence/r1-initial-probes.json) | Generic consume drops once; generic Copy+Drop conflict rejects E2004. Not a complete soundness matrix |
| R1 native C11 | [collect probes](evidence/collect-initial-probes.json) | Original callback shape accepted then access violation in both modes; corrected borrowed callback executes exit0. Initial artifact probes used existing canonical artifacts, not fresh certification |
| R1 trait argument validation | [fresh CLI verification](evidence/r1-bound-candidates-cli-verified.txt) | 7 fixtures in source-only/fresh-artifact sysroots: matching/multiple trait arguments, candidate inference rollback and mutable-reference impl pass; mismatching callback, borrowed type inheriting owned-type bound and shared reference satisfying mutable-ref impl reject E2021. Impl-header bounds and fully unconstrained candidate ambiguity remain audit tasks |
| R2 source interface identity | [HT7 verification](evidence/r2-interface-identity.txt); [combined run](evidence/r0-r2-current-regressions.txt); [reduced interface diff](evidence/r2-interface-diff.txt) | Shared canonical fingerprint, portable effects, manifest provider identity and Enum symbol kind reconcile the original HT7 mismatch. Fresh isolated HT7 and all 8 storage cases pass in the combined run |
| R2 native body boundary | [artifact regressions](evidence/r2-artifact-regressions.txt) | 7 provider linking + 9 metadata parity + 3 freshness + 5 strict rejection tests pass. Concrete artifact functions use object-backed identity rather than traversing their portable bodies as newly emitted definitions |
| R2 execution dependencies | [fresh CLI checkpoint verification](evidence/r2-execution-cli-checkpoint.txt) | Native call relinks changed dependency and executes new exit2; materialized generic, comptime and bundled-source bodies reject changed execution identity. Provider body hashing no longer includes global arena/session IDs. Comptime dependency selection is conservative, not yet precise |
| CALL-ARGUMENTS-v1 | [initial probes](evidence/r1-initial-probes.json) | Existing colon labels parsed but ignored: reordered call produces exit101. Full binding/default implementation pending |
| Unary logical Not | [initial CLI bound test](evidence/r1-trait-arguments-cli.txt) | Valid bound fixture encountered unsupported unary Not E2012. Bound fixture uses ordinary equality now; unary operator implementation/contract coverage remains an independent gap, not a passing feature claim |
| Combined R0–R2 run | [current regression run](evidence/r0-r2-current-regressions.txt) | 71 passed, 1 failed: conversions exposed first-self-match bound selection. Candidate search/rollback and reference mutability matching subsequently repaired; the failing suite and related compiler/driver cases pass in the specific reruns below |
| Bound-selection rerun | [driver regressions](evidence/r1-candidate-regressions.txt); [semantic/borrowck](evidence/r1-semantic-borrow-regressions.txt) | 38/38 driver cases, including all 6 numerics and C11, and 251/251 semantic/borrowck cases pass after candidate fix. These reruns supersede the exact conversion failure, not unrelated untested release gates |
| Artifact protocol | [library regressions](evidence/r2-llib-version-regressions.txt) | 13/13 library cases pass, including compiler header protocol v1 rejection before payload decode |
| Build surface | [workspace check](evidence/checkpoint-workspace-check.txt) | cargo check --workspace passes; existing compiler warnings remain. This is not a full workspace test or clean-checkout release gate |

Failure roots are not all compiler bugs. Preserve the named original failures,
reduce each one and close it at the correct boundary. Later logs supersede only
the exact cases and revision they actually run.

## Next dependencies

Verify R1 bound rejection and corrected C11 through CLI/source/fresh artifacts;
rerun related driver suites. Continue canonical interface reconstruction and
mixed graphs, associated types, generic drop matrix and borrowed provider
escape. Then implement adopted method policy and named/default calls, complete
diagnostics/provider/retained-contract matrices, reconcile current docs and take
R5 exact-candidate regression gates. No tag or merge recommendation yet.

## R1 ownership checkpoint — continued 2026-10-04

Implementation revision: `cd9987a` (`fix(ownership): elaborate local drop flags and repair aggregate cleanup`).

Status: **PARTIAL; release gate remains open.** New public CLI fixtures live in
`tests/luna/language/generic_drop`, orchestrated by `generic_drop_cli` using
fresh source-only and artifact-only sysroots. The imported user provider is
published in a separate CLI invocation; its `.ln` is removed in artifact mode.
The closure fixture remains a failing assertion, not an ignored or reclassified
test. No full workspace PASS is implied by this checkpoint.

- Fixed context leakage from an aggregate's expected type into unrelated
  tuple/array elements. Empty arrays retain their declared element type.
- Fixed managed-place overwrite: transfer the replacement first, destroy the
  initialized old value, then store. Self-assignment, moved-from initialization,
  Drop-owner field replacement and safe-reference replacement have runtime
  counter controls. Raw storage writes remain initialization without implicit
  old-value destruction under PTR-MEM-3, using type/place classification rather
  than a library function name. The initial overly broad overwrite change caused
  three Vec/Box heap-corruption regressions; preserve the failure log and use the
  passing specific rerun below to supersede only those failures.
- Added array drop discovery and a typed reverse-index loop, including empty
  arrays. Enum drop glue selects only the active payload; tuple payloads use
  their ordinary generic glue. Controls include multiple concrete payloads,
  nested wrappers, arrays, empty variants and multiple-field variants.
- Whole-local path-dependent cleanup now uses runtime initialization flags
  derived from ownership transitions and guarded CFG drops. The generated proof
  set suppresses only the guarded cleanup diagnostic; ordinary conditional use
  still rejects E3001. Both `check` and `build` use the same elaboration entry.
  Projected/partially moved aggregate cleanup remains separate audit work.
- Statement-only closure outputs infer void; closure calls check their actual
  parameter/return signature. Captures are stored into their environment, and
  the closure body loads the environment pointer before accessing its fields.
  The earlier native access violation is gone, but `closure_capture` still
  exits **3** in both modes: an unused owned capture is not destroyed. Closure
  environment destruction and repeated invocation after consuming a capture
  are not certified. The latter callable policy has been presented to the
  maintainer; do not infer a decision from silence.
- Backend drop calls reject a missing callee instead of silently succeeding.
  Pre/post-optimization MVIR verification now returns typed E6001 errors instead
  of returning successful compilation. Internal tests cover invalid IR, missing
  drop glue, valid external destructors and unhandled instructions.

Evidence:

- [Initial matrix](evidence/r1-generic-drop-initial.txt) and
  [overwrite intermediate run](evidence/r1-generic-drop-after-overwrite.txt)
  preserve the original failures and first reduced repairs.
- [Final CLI matrix](evidence/r1-drop-final-cli.txt): nine positive ownership
  fixtures execute exit0 in each mode; five negative fixtures reject in both
  `check` and `build`, in each mode. The closure fixture fails exit3 in both.
  Independent literal and trait-argument CLI suites pass. These counts describe
  fixture outcomes; the generic drop harness as a whole is **FAILED**.
- [Semantic/borrowck regressions](evidence/r1-drop-semantic-borrow-regressions.txt)
  pass. This run includes drop flags but predates the subsequent raw-storage
  classification adjustment in MVIR; it is not exact-final whole-workspace proof.
- [Driver regression failure](evidence/r1-drop-driver-regressions.txt) records
  three native heap-corruption failures introduced by the initial overwrite
  change. [Specific rerun](evidence/r1-drop-final-driver.txt) checks the corrected
  raw-storage boundary with fresh [canonical artifacts](evidence/r1-drop-final-sysroot.txt).
  The final rerun is **31/31 PASS** (backend 4, generic drop 7, subplace 10, Vec 10).
- [Workspace compile check](evidence/r1-drop-workspace-check.txt) passes with
  existing warnings. It is not the full workspace test/release gate.

The adopted per-call defaults decision remains unchanged. Method Option A and
CALL-ARGUMENTS-v1 implementation, closure cleanup, remaining R1/R2/R3 matrices,
documentation reconciliation and R5 release gates remain in the goal's scope.

Compiler Change
    Capability: Generic managed overwrite, aggregate destruction, conditional local cleanup and closure environment transfer.
    Why stdlib exposed it: The generic ownership audit and existing nested Vec/Box drop regressions exercise the same machinery.
    Why it is generic: Type/place identities and ownership transitions govern cleanup; raw initialization is distinguished from safe-place overwrite without container or function-name branches.
    User-defined type benefiting: Tracked<T>, Envelope<T>, Owner<T>, Choice<T> and Mixed<A, B> in an independently imported provider.
    Tests: generic_drop_cli; backend_fail_closed_tests; compiler_gap_c_gap_01_drop_mono_tests; p0b_drop_subplace_tests; stdlib_vec_element_drop_acceptance_tests.
    New intrinsic/lang_item?: NO
    Stdlib-specific branch?: NO

SKILL IMPACT: none for these fixes. Existing PTR-MEM-3, ownership, compiler
boundary and full-capability validation guidance already state the relevant
rules; the discovered implementation gaps do not redefine those contracts.

## Compiler–stdlib boundary report

Compiler Change
    Capability: Validate instantiated trait arguments, candidate inference rollback and reference mutability.
    Why stdlib exposed it: An iterator Filter callback had the wrong borrowed Item shape and compiled into a native access violation.
    Why it is generic: Candidates must satisfy self and trait arguments; failed candidates restore inference and equality obligations. Bound lookup does not silently inherit an owned-type impl through a reference. Reference patterns compare mutability rather than accidentally comparing lifetime IDs.
    User-defined type benefiting: Source implementing Produces<i32>, with matching/mismatching user callbacks.
    Tests: trait_argument_bounds_cli (7 positive/negative fixtures); existing collect C11 and standard conversions; semantic/borrowck regressions.
    New intrinsic/lang_item?: NO
    Stdlib-specific branch?: NO

Compiler Change
    Capability: Preserve source/artifact provider identity and distinguish object-backed calls from materialized execution dependencies.
    Why stdlib exposed it: Isolated RawTable source import invalidated fresh dependent artifacts; broader metadata tests exposed associated-type and mixed-graph failures.
    Why it is generic: Validated provider identity, canonical Enum kind, shared ABI serialization and explicit object-backed function identities apply to every provider.
    User-defined type benefiting: alpha::Item, assoc::Num/Producer and independent native/generic/comptime provider graphs.
    Tests: test_artifact_metadata_parity; test_freshness; test_strict_artifact_rejection; compiler_gap_c_gap_03_provider_linking_tests; artifact_execution_dependencies_cli.
    New intrinsic/lang_item?: NO
    Stdlib-specific branch?: NO

SKILL IMPACT: none for this checkpoint. Existing identity, generic boundary and
highest-public-test guidance already cover the verified mechanisms. The newly
adopted call syntax still requires grammar/skill reconciliation with its
implementation, rather than claiming availability from these contracts alone.

## Field cleanup and return transfer checkpoint — 3d69166

This extends the earlier generic-drop checkpoint; it does not close the whole
R1 matrix or change the adopted ownership contracts.

- The reduced [partial aggregate fixture](../../../tests/luna/language/generic_drop/partial_aggregate_cleanup.ln)
  originally built successfully and exited **2**: the moved first field was
  destroyed, but the remaining second field was leaked. Preserve the
  [before-fix result](evidence/r1-partial-aggregate-before.txt).
- MVIR now exposes cleanup of structs without a user destructor and tuples as
  reverse-order field cleanup, recursively. Move analysis can eliminate only
  transferred fields. Conditional initialization flags track these places as
  well as whole locals. User destructors remain indivisible, and moving a field
  out of a Drop-bearing owner still rejects. Raw-storage initialization remains
  separate from safe managed-place overwrite.
- Return values transfer before scope cleanup. Returning a field no longer
  exempts its entire containing owner from destruction. Explicit returns and
  tail expressions have controls with different generic field types. Ordinary
  whole-owner return/move controls remain in the matrix.
- Member lookup resolves already-bound inference variables both at the object
  and referent. Mutability checking resolves known bindings too; an inferred
  shared reference held in a mutable local still cannot mutate its referent.
- The permanent fixture covers nested tuple fields, conditional field/whole
  moves, field reinitialization, replacement of a partially moved whole value,
  imported generic field returns, reverse destructor order and borrowing a
  remaining disjoint field. New negatives reject whole-aggregate use after a
  partial move (E3001) and writes through inferred shared references (E2023).

Evidence boundaries:

- [Initial field-cleanup run](evidence/r1-partial-cli.txt) is intermediate
  debugging evidence: the fixture/provider was expanded while it ran. Do not
  use its counts as proof for a single source snapshot. It exposed the missing
  cleanup of the non-returned field (exit10).
- [Return-transfer run](evidence/r1-partial-return-cli.txt) passes the then-current
  partial cleanup fixture in both modes; closure capture remains exit3.
- [Expanded run](evidence/r1-partial-final-cli.txt) exposes inferred-reference
  member lookup rejection (E2001) in both modes. It is superseded only for that
  failure by the subsequent resolution fix.
- [Resolved CLI matrix](evidence/r1-partial-resolved-cli.txt) has **10 positive
  fixtures executing exit0 per mode** and **7 negative fixtures rejecting in
  check and build per mode** (20 executions and 28 negative commands). The
  generic-drop harness as a whole remains **FAILED**, solely for
  `closure_capture` native exit3 in both modes. Artifact execution dependency,
  literal typing and trait-argument suites also pass. This run precedes the
  final restoration of assignment spans on decomposed cleanup; it is not an
  exact-final diagnostic or full-workspace certificate.
- [Driver rerun](evidence/r1-partial-driver.txt) passes **47/47**: backend 4,
  generic drop 7, definition-site lifetimes 13, lifetime ABI 3, subplace drop 10,
  Vec element drop 10. [Semantic/borrowck rerun](evidence/r1-partial-semantic-borrow.txt)
  passes **251/251**. Both precede the later inferred-member/mutability fix.
- [Workspace compile check](evidence/r1-partial-workspace-check.txt) passes on
  the final source, including restored spans; existing warnings remain. The
  completed [full workspace summary](evidence/workspace-3d69166.summary.txt)
  ([lossless log](evidence/workspace-3d69166.txt.gz)) on `3d69166`
  exits 101, with 1,276 passed, 6 failed and 1 ignored.

The fresh-artifact controls prove the repaired struct/tuple cases. Enum-pattern
partial cleanup, indexed moves, closure environment destruction, repeat-call
ownership, async cancellation and the remaining R1/R2/R3 matrices still require
separate verification. The current artifact compiler compatibility revision
must also be reviewed before freeze: existing native/portable bodies compiled
with older cleanup cannot be repaired merely by loading them with a new compiler.

### Updated native CI evidence

[GitHub metadata snapshot](evidence/ci-after-drop-checkpoint.json) records the
completed workflows at `5ef8137073c7a5416ad8a271b91ce3455675b00c`:
[platform run](https://github.com/17thedevv/luna/actions/runs/37190486969) and
[formatting run](https://github.com/17thedevv/luna/actions/runs/37190486956).
Runtime ABI and Whole-File I/O pass on Ubuntu and macOS Intel; Ubuntu ASan and
formatting on both hosts also pass. Full workspace regression fails in both
workflows. The authenticated [Ubuntu platform workspace job log](evidence/ci-workspace-5ef8137-ubuntu.txt.gz)
([retrieval and hash record](evidence/ci-workspace-log-checkpoint.json))
shows `cargo test --workspace -- --test-threads=1` stopping at the generic-drop
CLI harness: `closure_capture` exits 3 in both modes. Its fail-fast result is
not a complete failure census of later suites. The formatting workflow's
workspace log has not been independently downloaded. These results supersede
the older Whole-File I/O CI failures only; they are not evidence for `3d69166`
or the later async/borrow fixes.

Compiler Change
    Capability: Partial struct/tuple cleanup, per-place conditional initialization, return ownership transfer and resolved member/mutability lookup.
    Why stdlib exposed it: The generic ownership audit and nested resource/drop regressions exercise the same cleanup machinery as library ownership abstractions.
    Why it is generic: Semantic type identities, field projections and ownership transitions govern cleanup; no provider, container or function-name branches were added.
    User-defined type benefiting: Split<A, B>, Tracked<T> and Trace<T> in an independently imported provider.
    Tests: generic_drop_cli; backend_fail_closed_tests; compiler_gap_c_gap_01_drop_mono_tests; p0b_drop_subplace_tests; lifetime_def_site_acceptance_tests; lifetime_relation_abi_acceptance_tests; stdlib_vec_element_drop_acceptance_tests.
    New intrinsic/lang_item?: NO
    Stdlib-specific branch?: NO

SKILL IMPACT: none. Existing guidance already requires remaining-live-value
cleanup, ordinary move/borrow/mutability enforcement and full capability
verification. These implementation fixes do not establish a new language rule.

## Async cancellation and closure escape checkpoint

Implementation: `363eaf51f5d4bd59f7060cabc624ad6a6473dbea`, after the completed
`3d69166` full-run baseline. The [checkpoint record](evidence/async-closure-checkpoint.json)
pins Git blob identities and evidence hashes. This checkpoint remains **PARTIAL**.

The six baseline failures are recorded separately rather than subtracting
focused reruns from the full-run result:

| Suite / failure | Baseline observation | Later evidence and remaining limits |
|---|---|---|
| `generic_drop_cli` / `generic_owned_values_drop_once_across_control_flow_and_provider_modes` | `closure_capture` native exit3 in source and artifact modes | Still fails: unused owned closure captures are not destroyed |
| `adv_async_dyn_tests` / `test_adv_14_future_cancellation_at_suspended_state_with_dyn_trait` | Synthesized cleanup calls short name `drop`; backend rejects missing callee | Canonical glue and environment handle repaired; 16/16 driver cases pass and new initial-cancellation native fixture passes in both modes; conditional/projected suspension state is not certified |
| `sem_maturity_phase2b` / `regression_controls::e3005_02_closure_local_borrow` | Returning a closure borrowing a local was accepted | Return-transfer `Assign` lost capture facts. Propagation through Assign and canonical Store/Load repairs direct/local returns. Move-reference capture also preserves its referent loans; negative controls reject E3005 |
| `stdlib_option_result_borrow_acceptance_tests` / `test_opt_res_source_and_llib_parity` | LocalBorrowEscape when materializing a borrow of primitive constant `DUMMY` | Open: establish the intended scalar constant storage/borrow contract before changing implementation or oracle |
| `whole_file_io_v1_acceptance_tests` / `whole_file_io_source_and_fresh_artifact_parity` | Copied source root lacks language-contract `.ln`; bootstrap cannot find `__lang_drop` | Isolated full suite passes 3/3 without environment override; one case also passes with override. This does not establish the original root cause |
| Same suite / `copy_file_propagates_destination_write_failure_in_source_and_fresh_artifact_modes` | Same missing bootstrap provider | Source inventory loss remains an R0 harness investigation; an environment override is not a confirmed cause |

### Repairs and public controls

Synthesized async resume/drop functions now load the incoming environment
handle from the ABI parameter slot, then cast it to the concrete environment
pointer before projecting fields or freeing memory. Ordinary owned values use
the same canonical glue identity as normal MVIR cleanup; recursive aggregate
cleanup receives an address. Future cleanup retains its originating-function
mechanism, whose cross-provider identity coverage still requires audit.
Initial/suspended cleanup lists are deterministic and reversed.

Closure capture facts follow Assign, canonical storage and Load; control-flow
merge unions possible capture modes. Load retains closure provenance. Moving a
reference or borrowed aggregate into a closure carries its existing loans into
the environment; a move does not extend the referent's lifetime. Return escape
checks cover this provenance even when the callable signature erases capture
details. The permanent fixtures use direct and locally stored returns, moved
references to local data, and valid scoped/repeated invocation. This does not
claim complete aggregate/interprocedural closure provenance or destruction.

- [Before-fix move-reference probe](evidence/r1-closure-move-borrow-before.txt):
  invalid program was accepted by `check`, exit0.
- [Final public CLI matrix](evidence/r1-closure-move-borrow-cli.txt): 12 positive
  fixtures per mode execute exit0; 10 negative fixtures per mode reject in both
  check/build (24 successful executions, 40 correct rejections). The harness
  exits 101 solely for `closure_capture` native exit3 in both modes. Sysroots
  and imported resource artifacts are freshly built; artifact roots exclude
  their source providers.
- [Final driver/borrowck regression](evidence/r1-async-closure-driver-final.txt):
  117 passed, 0 failed, 1 ignored (borrowck 12, driver lib 17, async/dyn 16,
  semantic maturity 72). The ignored case is unchanged.
- [Workspace compile](evidence/r1-async-closure-workspace-check.txt): exit0,
  existing warnings remain. No full workspace test or CI PASS on this
  checkpoint is claimed.
- Intermediate logs retain the discovered failures and revision boundaries:
  [initial future CLI](evidence/r1-future-initial-cli.txt),
  [direct-return driver](evidence/r1-closure-transfer-driver.txt),
  [local-return failure](evidence/r1-closure-provenance-cli.txt),
  [storage CLI](evidence/r1-closure-storage-cli.txt),
  [storage driver](evidence/r1-closure-storage-driver.txt),
  [async unit regression](evidence/r1-async-canonical-glue.txt),
  [async integration regression](evidence/r1-async-driver.txt).
  The initial async unit command filtered out all 16 integration cases; only
  the explicitly rerun integration command establishes their PASS.
- Whole-File I/O focused controls:
  [without override](evidence/r0-whole-file-without-env-override.txt),
  [with override](evidence/r0-whole-file-env-override-control.txt).

Closure destruction/invocation ownership, suspended-state conditional/partial
cleanup, scalar const borrowing and source inventory isolation remain open.
The adopted method policy and named/default calls remain implementation tasks;
R2/R3/R4 matrices and exact-candidate R5 gates remain in the goal.

Compiler Change
    Capability: Canonical aggregate cleanup from synthesized async functions and preservation of closure capture/loan facts through value and storage transfers.
    Why stdlib exposed it: The ownership matrix and retained async/closure contracts exercise generic user-defined resources, independent of container names.
    Why it is generic: Semantic types, canonical instances, ABI parameter slots, places and provenance govern the repairs.
    User-defined type benefiting: Tracked<T>, Split<A, B>, Trace<T> and closures capturing arbitrary reference-bearing values.
    Tests: generic_drop_cli, adv_async_dyn_tests, sem_maturity_phase2b, borrowck and driver lib regressions.
    New intrinsic/lang_item?: NO
    Stdlib-specific branch?: NO

SKILL IMPACT: none. The existing guidance already requires retained referent
lifetime, ownership transfer and canonical cleanup. No new rule is introduced.

## Pre-repair artifact rejection checkpoint

Implementation: `f5936c3344788168320dfd4b661532c2cfad8a8d`.
Compiler header version is now **3**. Versions 1 and 2 reject before payload
decoding, so old native and portable bodies cannot bypass repaired ownership
cleanup merely because their source/interface hashes still match. Format and
MVIR versions are unchanged. Import still rejects selected invalid artifacts;
rebuilding remains an explicit build-tool operation.

- [Compiler build](evidence/r2-cleanup-compat-build.txt): exit0.
- [Canonical sysroot rebuild](evidence/r2-cleanup-compat-sysroot.txt): exit0.
- [Reader/driver regressions](evidence/r2-cleanup-compat-reader-driver.txt):
  35/35 passed (driver lib 17, strict rejection 5, sysroot invariants 6,
  artifact reader/metadata 7). Reader control checks both previous compiler
  versions before payload decode.
- [CLI execution-dependency regression](evidence/r2-cleanup-compat-execution-cli.txt):
  1/1 passed with fresh providers; ordinary native relinking and materialized
  body invalidation remain distinct.
- Generated module documentation is reconciled. Documentation validator:
  172 documents, 29 site pages, 1,143 local links, no errors.

These are compatibility and focused verification results, not full-workspace
or release PASS. Closure cleanup, remaining ownership matrices, adopted method
selection and named/per-call default arguments, R3/R4 and final R5 gates remain
open. At this checkpoint the module-constant storage decision was still pending.
The subsequent adopted decision and implementation gap are recorded below.

SKILL IMPACT: none. Existing artifact guidance already requires compiler
identity validation and fail-closed rejection without rebuild/fallback.

## New contract and method-policy counterexamples

The maintainer adopted [MODULE-CONST-STORAGE-v1](../../spec/0.1/module-const-storage-v1.md):
module-level constants provide immutable storage lasting for the program's
lifetime when borrowed; local constants remain scoped. Implementation is
pending. The [public pre-fix controls](evidence/r1-module-const-before.txt)
on `f5936c3` reject the required-valid module reference with E3005; local escape
rejects E3005 and mutable module borrow rejects E2023 as required. Current
`generate_lvalue` materializes a constant into a function Alloca, which is the
first incorrect representation for the new rule. The fix must create actual
typed immutable target storage with declaration identity and portable initializer
dependencies; setting `ValueOrigin::Global` on that Alloca would be unsound.

The [pre-fix checkpoint](evidence/const-method-pre-fix-checkpoint.json) pins the
CLI and fixture/evidence hashes. The adopted method policy has independent
reduced counterexamples:

- [Parsed pre-fix check controls](evidence/r2-method-policy-before-valid.txt):
  `ambiguous_traits.ln` is incorrectly accepted instead of E1008. Both qualified
  calls and the imported-inherent/local-trait control are accepted.
- [Native controls](evidence/r2-method-policy-native-before.txt), with a
  [mode/revision record](evidence/r2-method-policy-native-before.json): qualified
  calls select the correct trait implementations, and the imported inherent
  method wins against a local trait in this reduced case. All four runs exit0.
  The provider is freshly built for artifact mode and its adjacent source is
  absent there. Sysroot artifacts are existing canonical version-3 artifacts;
  these probes are not a complete fresh-isolated parity certificate. Do not
  report this passing local/imported control as a newly reproduced defect merely
  because source contains separate lookup loops.
- [Applicability controls](evidence/r2-method-applicability-before.txt):
  `inapplicable_inherent_bound.ln` selects an inherent method requiring
  `T: Marker` for `i32` with no matching impl. Check/build accept, native exits1
  because the required applicable trait fallback was not selected.
  `inapplicable_inherent_arity.ln` passes check but build rejects E6001 at LLVM
  verification for an incorrect argument count. The applicable trait candidate
  has the correct arity. Backend fail-closed exposes this defect; the candidate
  selection/typecheck boundary must reject that inherent candidate earlier.

Permanent fixtures are under `tests/luna/language/method_policy/` and
`tests/luna/language/module_const_storage/`. These are pre-fix counterexamples,
not a new passing acceptance harness.

Next implementation order:

1. Implement module-constant storage as a generic typed IR/backend/interpreter
   capability with correct global provenance and source/artifact preservation;
   retain local/mutability negatives and the original borrowed Option/Result
   regression. Audit compiler/MVIR/metadata compatibility for any new portable
   representation. No symbol-name or fixture-specific branch.
2. Collect canonical local/imported method candidates together; probe receiver,
   arity, generic bounds and access without leaking speculative inference or
   equality obligations. Rank applicable inherent candidates before traits;
   deduplicate declaration identity and diagnose distinct trait ambiguity with
   E1008/related spans. Expected return type must not resolve that ambiguity.
   Keep passing qualification/imported-inherent controls and test reversed order,
   visibility, explicit method generics and artifact metadata paths.
3. Continue closure destruction/conditional future cleanup and the remaining
   R1/R2/R3 matrices; implement named/default calls under their adopted per-call
   definition-scope contract, reconcile R4 and take exact-candidate R5 gates.

SKILL IMPACT: none. Existing skills defer semantic authority to the versioned
spec/adopted amendments and require generic storage, provenance, candidate
validation, negative controls and source/artifact parity. The new amendment
belongs in that specification rather than being duplicated as skill authority.

## Module-constant storage implementation checkpoint

Implementation: `bdbd8b1` (`fix(compiler): give module constants immutable program storage`).
The adopted module/local distinction now has an actual storage representation:
module constants lower to `StaticAddress` with a typed structural initializer,
while local constants retain scoped storage. LLVM emits immutable target data;
the interpreter uses a separate immutable arena that survives calls and rejects
write, mutable borrow, drop and deallocation. No stack Alloca is relabelled Global.

Provider reconstruction now preserves evaluated local declaration constants,
comptime values and module-storage classification, including private constants
used by exported native and generic bodies. Private constants remain inaccessible
to consumers. Initializer payloads retain structural values rather than source
semantic IDs or VM addresses. Enum strings carry target relocations. Reference
array indexing also loads the reference binding before offsetting array storage.

The portable MVIR representation changes: **compiler version 4 and MVIR version
4**, with format 2 and semantic metadata 3 unchanged. Earlier compiler/MVIR bodies
reject before payload decoding; rebuilding is explicit. New reader controls
reject truncated and trailing initializer payloads.

- [Focused CLI matrix](evidence/r1-module-const-cli.txt): 1/1 harness PASS,
  six native runs and twenty check/build negative observations. Independent
  source-only and fresh-artifact-only sysroots contain all 49 providers. Project
  providers are relocated and adjacent source is absent in artifact mode.
  Coverage includes repeated references, private constants, distinct providers,
  native/generic shared storage, wide integers, Unicode char/string escapes,
  float, tuple/struct/array and nested enum data with string relocation.
  Local scalar/aggregate escape rejects E3005, mutable borrow E2023, private
  access E1001, and a comptime VM-pointer escape E4005.
- [IR/VM/reader regressions](evidence/r1-module-const-ir-vm.txt): 24/24 PASS
  (9 library reader tests, 6 artifact integrations, 7 comptime regressions and
  2 immutable-storage VM controls).
- [Original borrowed Option/Result suite](evidence/r1-module-const-option-result.txt):
  9/9 PASS, including `test_opt_res_source_and_llib_parity`.
- [Canonical sysroot](evidence/r1-module-const-sysroot.txt): all 49 providers
  rebuilt successfully with version-4 artifacts.

The completed [workspace summary](evidence/workspace-module-const.summary.txt)
and [lossless log](evidence/workspace-module-const.txt.gz) record **1,284 PASS,
3 FAIL and 1 ignored**, across three failing targets. The process wrapper
returned 1; the individual cargo exit was not retained. The
[checkpoint](evidence/module-const-checkpoint.json) pins `bdbd8b1`, source blobs,
fixtures and evidence. This was a development-tree run started before commit,
with formatting-only edits to four new Rust files after compilation, not an
exact clean-checkout release certificate. The CLI matrix in this run additionally
checks comptime field and array-element reads added after the first focused run.

The three failures are:

1. `generic_drop_cli`: moved closure capture cleanup still exits3 in source and
   artifact modes; unresolved.
2. `adv_comptime_dyn_tests::test_adv_comptime_15_array_size_from_comptime_associated_type`:
   newly exposed storage lowering regression. Early array-size evaluation has
   no expression-type entry; using that entry's default Void instead of the
   checked constant declaration type rejects a valid initializer.
3. `stdlib_hashmap_storage_acceptance_tests::test_ht7_source_vs_llib_parity`:
   its fresh provider build fails with missing-file `os error 2`. The retained
   build root has artifacts but no `lang/*.ln`; the actor/root cause is not
   established. Passing isolated results cannot close this failure.

The earlier ADV-14 async, closure escape, Option/Result borrowed parity and
Whole-File I/O suites PASS in this workspace run. That is current run evidence;
it does not identify the cause of historical missing-source failures or certify
all ownership/async paths.

Follow-up implementation `d333824` uses the checked declaration type for static
storage and loads, including before expression typing. The permanent CLI
comptime fixture includes an associated-type constant as an array length.
[Affected comptime suite](evidence/r1-module-const-array-repair.txt): **16/16 PASS**.
[Final CLI matrix](evidence/r1-module-const-final-cli.txt): **1/1 PASS**, six native
runs and twenty correct negative check/build observations across source and
fresh artifacts, now including that array-length case.
[Final workspace check](evidence/r1-module-const-final-check.txt): **exit0**.
The [follow-up checkpoint](evidence/module-const-array-repair-checkpoint.json)
pins this revision and its focused verification. A full workspace has not been
rerun on this follow-up; the prior full FAIL stays FAIL. Focused PASS does not
close target coverage or R5. Existing closure destruction, method
applicability/ambiguity, named/default calls and remaining
ownership/provider/diagnostic matrices stay open.

    Compiler Change
        Capability: Immutable program-lifetime module-constant storage.
        Why stdlib exposed it: Borrowed Option/Result parity returns a reference to a module constant.
        Why it is generic: Typed static data, declaration identity and provider reconstruction apply to arbitrary user declarations.
        User-defined type benefiting: The permanent user provider's Pair, Nested, Choice and TextChoice values.
        Tests: module_const_storage_cli, static_storage, artifact reader roundtrip, original Option/Result regressions.
        New intrinsic/lang_item?: NO
        Stdlib-specific branch?: NO

SKILL IMPACT: **CORRECTION** in `mellis-grammar`: array types now use `[T; N]`
as already required by the versioned syntax; the illustrative private field
uses explicit `private`. Parser semantics are unchanged. The skill and related
semantic guidance were checked against the canonical syntax/visibility rules;
no task status or new language authority is added to skills.


## R2 method/binder checkpoint — 2026-10-05, not release approval

The maintainer adopted independent impl/method binders. Implementation
`fbf4e72` removes name/position aliasing between unrelated binders, carries each
selected checked impl header through provider reconstruction, and checks all
applicable inherent candidates before trait candidates. Multiple applicable
trait methods report E1008; explicit qualification selects the trait contract.
Expected result types do not choose between ambiguous unqualified candidates.
Vec element Eq/Clone requirements now reside on constrained impls; method type
parameters remain independent. RawTable accessors no longer inherit hashing
bounds. The duplicate String.push_string declaration was removed.

The first [focused matrix](evidence/r2-method-final-cli.txt) passed 64 native
runs and 88 negative check/build observations, plus trait-argument parity.
[Workspace check](evidence/r2-method-final-check.txt) and
[canonical sysroot build](evidence/r2-method-final-sysroot.txt) passed.
The subsequent full run on `fbf4e72` is **FAIL**: **1,270 PASS, 18 FAIL and
1 ignored**, cargo exit101, 14 failing targets. Its
[start record](evidence/workspace-method-start.json),
[exit record](evidence/workspace-method-exit.json),
[summary/inventory](evidence/workspace-method-summary.json) and
[lossless log](evidence/workspace-method.txt.gz) preserve that result.
Cargo rebuilt the CLI during the workspace run; before/after binary hashes
are recorded separately. Generated test_model.mvir was the only tracked file
changed by that run. This is development-checkout regression evidence, not
clean-checkout or advertised-target R5 certification.

Follow-up repairs address three independent compiler counterexamples:

- Blanket impl matching must prove the matched impl's own recursive bounds,
  associated-type equalities and exact trait arguments. A recursive assumption
  does not prove itself. Caller binders are rigid within this proof; candidate
  failures roll back inference/obligations/diagnostics. Qualified trait calls
  also require an applicable impl with satisfied premises. Impl and method
  associated equalities are retained, including for generic callers.
- Array types participate structurally in impl matching, overlap detection and
  occurs checking. Reference overlap compares mutability, not lifetime IDs.
- Calling through a mutable reference uses its capability and does not require
  a mutable binding for the reference variable. Owned immutable receivers and
  shared references still reject mutable calls.

The driver now stops after semantic errors before monomorphization, preventing
secondary E5001 diagnostics from hiding the primary failure or differing across
source/artifact modes. Historical fixtures were aligned with the adopted
policy: Display/Debug and conversion collisions use qualification; resize no
longer redeclares the impl binder; mutated owned locals use rw; diagnostic
oracles check typed codes. Ordinary user trait methods named drop are callable;
actual std::Drop destructors remain rejected. No safety rejection was removed
to accommodate a positive fixture.

**Artifact compiler version 6**, MVIR4, metadata3 and format2. Earlier compiler
bodies reject explicitly; no automatic rebuilding is added. The
[final canonical build](evidence/r2-method-proof-sysroot.txt) rebuilt all 49
providers. Public metadata grouping/fingerprint identity still needs a separate
per-impl-header audit; portable AST reconstruction passing does not close it.

Follow-up implementation `c268372` is pinned by the
[checkpoint](evidence/method-proof-checkpoint.json), with source blobs and
evidence/binary hashes.

Verification at the follow-up source state:

- [Final CLI suites](evidence/r2-method-proof-final-cli.txt): five targets PASS;
  method matrix **88 native runs and 176 correct negative observations** across
  source/fresh-artifact and declaration order, including relocated custom
  providers. Associated premises, cycles, array lengths/overlap, binder identity,
  mutable-reference binding and real/ordinary drop controls are covered.
  Char/Unicode, formatting, receiver diagnostics and trait-argument parity PASS.
- [Semantic suite](evidence/r2-method-proof-semantic.txt): **191/191 PASS**.
- [Artifact reader](evidence/r2-method-proof-artifact.txt): **9/9 PASS**, including
  rejection of earlier compiler versions before decoding bodies.
- [Workspace check](evidence/r2-method-proof-check.txt): exit0.
- [Earlier affected driver suites](evidence/r2-method-bound-driver.txt):
  **95/95 PASS** across ten targets, before the associated/rigid bound-proof
  follow-up. This covers Whole-File I/O, iterator consumers, resize/clone
  ownership, conversions, namespace closure, async/dyn, alias evaluation order
  and semantic gates. It is not final-state full regression evidence.

**R2 remains PARTIAL.** A new permanent required-negative fixture,
`tests/luna/language/generic_typing/rigid_return_required_reject.ln`, proves
that the general unifier still accepts an unrelated i32 as a generic T:
`wrong<bool>()` checks, builds and exits0. Strict trait proof does not repair
ordinary expression/return typing. A required-positive nested struct-literal
fixture also rejects at parsing. These are retained counterexamples, not
passing certificates or permission to weaken the contract. The bound prover
currently has a 64-frame expansion limit reported as unsatisfied evidence;
limit diagnostics/deeper valid graphs need audit. Broader associated projection
and fallback lookup domains remain open.

Next: repair ordinary rigid generic typing, then nested-expression parsing and
per-header canonical metadata; continue moved closure destruction and the
adopted named/default argument contract. R3/R4 target/provider/diagnostic
coverage and exact-candidate R5 remain required. No merge or release tag.

    Compiler Change
        Capability: Applicable method selection, independent binder/header identity and recursive trait/associated premise validation.
        Why stdlib exposed it: Vec constraints, formatting collisions, mutable iteration and Whole-File I/O require ordinary generic/receiver contracts.
        Why it is generic: Declaration identities, structural matching and trait evidence apply to arbitrary user types; invalid typing stops before mono.
        User-defined type benefiting: Pair, Wrapper, AssocWrapper, ArrayHolder, Counter and custom provider traits in the CLI matrix.
        Tests: method_resolution_cli, trait_argument_bounds_cli, char/format CLI, semantic/reader suites and affected driver regressions.
        New intrinsic/lang_item?: NO
        Stdlib-specific branch?: NO

SKILL IMPACT: **REFINEMENT** in luna-semantic-compliance: independent binders,
individual checked headers, complete impl premises and reference write
capability are reusable guidance under existing contracts. Related grammar,
boundary and validation guidance were checked for conflicts; skills acquire
no task status, test counts or freeze authority.

## R2 rigid generic bodies and nested literals — 2026-10-05 follow-up

The maintainer's independent-binder decision also exposes a distinct correctness
bug in ordinary unification. At the preceding checkpoint, `fn wrong<T>() -> T
{ return 7i32; }` could be instantiated as bool, checked, compiled and executed.
The general unifier now distinguishes rigid declaration binders from solvable
inference variables. An unrelated concrete type or different declaration binder
rejects with E2001. Trait-bound method candidates instantiate trait parameters
and `Self` before checking their receivers, rather than relying on the old
wildcard behavior. No spelling-based binder alias or library-name branch is
introduced.

The nested `Wrapper<Item> { item: Item {} }` parser counterexample is also
repaired. Empty struct literals compose with an outer `}`, operators and tail
expressions; conditional/match subjects retain their separate ambiguity rule.
Parser controls inspect AST shape and preserve condition blocks. The CLI matrix
includes nested nonempty data and a generic provider call containing the nested
literal, rather than stopping at parse acceptance.

Compatibility: **artifact compiler7**, format2, metadata3 and MVIR4. Reader
controls reject compiler1–6 before payload decoding, including artifacts that
could contain native bodies accepted under wildcard generic typing. Canonical
sysroot regeneration is explicit; stale artifacts are never silently rebuilt
by the importing compiler.

Verification and provenance:

The [rigid generic checkpoint](evidence/rigid-generic-checkpoint.json) pins
`1bf0365879707ab11f665aa6a22cf776f69ff6ff`, source blob identities, CLI/runtime
hashes, log hashes and the initial FAIL/corrected-rerun boundary.

- [Parser regressions](evidence/r2-rigid-parser.txt): **37/37 PASS**, including
  nested/tail/operator AST controls and condition-block preservation.
- [Semantic and artifact internals](evidence/r2-rigid-internals.txt): **206/206
  PASS** (semantic191 and artifact15).
- [Canonical sysroot](evidence/r2-rigid-sysroot.txt): all49 providers built with
  compiler7. [Workspace check](evidence/r2-rigid-check.txt): cargo exit0.
- [Initial focused CLI run](evidence/r2-rigid-cli-initial.txt): cargo **exit101**,
  preserved as FAIL. Generic, method, const and char matrices passed, and the
  positive formatting test passed, but parallel formatting tests selected the
  same timestamp-based temporary root on Windows. The negative formatting test
  failed with `AlreadyExists`; cargo stopped before the trait-argument target.
- The shared CLI provider harness now adds a process-local atomic counter to its
  temporary-root identity. The [corrected formatting/trait rerun](evidence/r2-rigid-format-trait.txt)
  records **3/3 PASS**, cargo exit0; it does not rewrite the preceding FAIL.

`generic_typing_cli` verifies **4 native executions and 24 E2001 observations**
through check/build, with no rejected executable emitted and no E5001 cascade.
Its user-defined provider is published, source removed in artifact mode and
relocated before use. Controls cover identity/constructor/borrowed returns,
same-name impl/method binders, alpha-renamed trait-method binders, multiple
concrete instances, generic bound dispatch, and invalid return/initializer/call
argument/field types. The method matrix retains **88 native and 176 typed
negative observations**, in both provider modes and declaration/import orders.
Module-constant regressions also pass on the new compiler.

This is a focused checkpoint, **not R2 completion or release approval**. The
full workspace result at fbf4e72 remains FAIL. Canonical metadata grouping and
fingerprints, associated projection/fallback domains, the 64-frame proof-limit
diagnostic, moved closure destruction, adopted named/default calls and R3–R5
remain open. A separate standalone `boolean_not_observation.ln` preserves the
observed E2012 rejection of logical-not for the operator coverage audit; it is
outside this passing generic matrix and requires contract reconciliation.

    Compiler Change
        Capability: Rigid generic body typing and composition of nested struct literals.
        Why stdlib exposed it: Independent Vec impl/method bounds exposed reliance on wildcard generic unification.
        Why it is generic: Declaration/inference identities and syntactic expression contexts govern the change; no provider, container or method-name knowledge is added.
        User-defined type benefiting: Cell<T>, Wrapper<T>, Choose<V> and independent generic functions in the relocated provider matrix.
        Tests: generic_typing_cli; method_resolution_cli; module_const_storage_cli; parser, semantic and artifact reader suites; char/format/trait CLI regressions.
        New intrinsic/lang_item?: NO
        Stdlib-specific branch?: NO

SKILL IMPACT: **REFINEMENT** in luna-semantic-compliance: declaration binders
remain rigid in generic bodies as well as trait proof. Related grammar,
capability-validation and boundary guidance were checked; no conflicting rule,
task count or freeze claim was added to skills.

## Full workspace at de9977d and additional reducers — 2026-10-05

The [full-run summary](evidence/workspace-rigid-summary.json),
[start record](evidence/workspace-rigid-start.json),
[completion record](evidence/workspace-rigid-exit.json) and
[lossless log](evidence/workspace-rigid.txt.gz) pin
`de9977d71dfb52a14617faf469441530dddb597e`. All targets were prebuilt before
`cargo test --workspace --no-fail-fast`; CLI and runtime hashes are identical
before and after execution. The result is **1,290 PASS, 1 FAIL, 1 ignored**,
cargo **exit101**. The sole failing target is `generic_drop_cli`: unused owned
closure captures are not destroyed. The generated `test_model.mvir` changed
during tests; no production source changed during this run. Earlier full FAIL
records remain retained; neither this result nor focused passes approve release.

Independent probes during the full run kept the compiler fixed and exposed
further requirements outside that workspace matrix:

- [Concrete inherent dispatch](../../../tests/luna/language/method_policy/concrete_inherent_dispatch.ln)
  builds successfully but exits1 instead of required exit0. Inherent methods on
  `Holder<i32>` and `Holder<bool>` share the same backend symbol despite
  different bodies. This is a **correctness blocker**. The
  [different-return provider reducer](../../../tests/luna/language/method_policy/concrete_inherent_distinct_returns.ln)
  passes check but fails LLVM verification with E6001. Both functions are named
  `_MMN5multi6HolderE4read`; nominal path plus method name does not distinguish
  concrete self types. The selected checked header must govern instance naming.
- [Fingerprint observations](evidence/impl-bound-observations.json) show that
  changing a public impl bound from `T: First` to `T: Second` changes source and
  execution fingerprints while preserving the same interface fingerprint,
  `e38bf0636d56fccd276b758cae4bff5cc25b596856d3020215ee8c54bfcd18df`.
  Canonical public metadata must retain these constraints and individual
  applicable self headers. Portable AST rechecking is not a substitute for a
  complete public dependency identity.
- The same probe reports a Windows-GNU triple with manifest `object_format:
  ELF`, while object bytes start `64 86` (AMD64 COFF). The driver currently
  hardcodes ELF, pointer width64 and empty ABI/endianness fields. Target
  contract/data-layout work remains required under R3.

Next implementation order: repair concrete instance identity and remove
spelling-based mangler substitution, verify native/source/relocated artifact
controls, then repair canonical constraint/header metadata. Continue closure
cleanup under its ownership contract, named/default calls and the remaining
R3–R5 acceptance obligations. The goal remains active; no merge/tag.

## Concrete inherent instance identity follow-up — 2026-10-05

Method ABI naming now uses the owning declaration's checked self type. The
full shape of `Holder<i32>` and `Holder<bool>` survives into the backend symbol;
method generic arguments remain independent. Trait method naming uses the exact
owning trait entry rather than searching another impl by nominal head or return
type. Missing ownership/header evidence fails closed. The mangler's remaining
generic substitution by parameter spelling was removed: binder identity is the
only substitution key.

Compatibility: **artifact compiler8**, format2, metadata3 and MVIR4. Native
method ABI names changed, so compiler1–7 artifacts reject before payload decode.
The canonical sysroot was explicitly rebuilt. No stdlib/provider name branch,
new language keyword or intrinsic was added.

- [Semantic/artifact internals](evidence/r2-concrete-internals.txt): **206 PASS**.
- [Canonical sysroot](evidence/r2-concrete-sysroot.txt): **49/49 built**.
- [CLI matrix](evidence/r2-concrete-cli.txt): **6/6 harness cases PASS**, cargo
  exit0. Expanded method coverage includes **96 native executions and 176 typed
  check/build rejections**. A sixth user provider exercises same-name methods
  on two concrete self types, distinct return types and independent method
  binders; publishing, source removal, relocation and reversed declarations
  are checked. Generic typing, module constants, formatting and trait arguments
  also pass on this compiler.
- [Linking/identity/drop regressions](evidence/r2-concrete-driver.txt): **34/34
  PASS**, including nominal/generic method mangling, provider linking, function
  pointers, generic drop and metadata round trips.
- [Workspace check](evidence/r2-concrete-check.txt): cargo exit0.

The direct native reducer changes from exit1 to required exit0; the distinct
return-type library reducer now builds instead of emitting E6001. The earlier
full workspace run remains FAIL and predates this ABI change. This closes the
two concrete method reducers, not all identity/fingerprint domains or release
gates. Canonical public constraints/headers, target metadata, closure cleanup,
named/default calls and the remaining R3–R5 scope stay open.

    Compiler Change
        Capability: Distinct canonical backend identities for concrete inherent methods, exact trait ownership and identity-only generic substitution.
        Why stdlib exposed it: Method/binder validation needed the same generic mechanism for arbitrary library implementations.
        Why it is generic: Checked impl self types and declaration binders determine identity; library container/provider names are absent.
        User-defined type benefiting: Holder<i32>, Holder<bool>, independent map<T>/map<U> and reversed relocated providers.
        Tests: method_resolution_cli, generic_typing_cli, module_const_storage_cli, formatting/trait CLI, semantic/reader, mangling/linking/function-pointer/drop/metadata suites.
        New intrinsic/lang_item?: NO
        Stdlib-specific branch?: NO

SKILL IMPACT: none for this follow-up. Existing semantic and capability guidance
already requires identity-based substitution and distinct backend instances.


## Public generic contracts and independent binders — 2026-10-05

The maintainer adopted independent impl/method binders; element constraints
belong on constrained impls rather than joining two binders by spelling. Vec's
Eq/Clone impl constraints remain in place. Canonical public metadata now retains
individual checked impl headers, method contracts, declaration-owned generic
bounds, trait arguments and associated equalities/definitions. Binder identities
use owner plus parameter position. Type interning and header ordering exclude
session allocation IDs. Function-body locals are excluded from public exports.
Changing a public bound invalidates dependent artifacts; renaming binders,
reordering declarations or changing a body while preserving its public effects
does not change the interface fingerprint.

Validation also exposed missing registration of nominal/trait/method bounds,
missing inferred-constructor bound checks and forward impl indexing. These now
use declaration-owned generic machinery. Unknown/non-trait/inaccessible bounds,
invalid trait argument arity and unknown associated bounds reject. Strict bound
lookup exposed slice's undeclared std::Copy dependency; its source now explicitly
imports the ordinary copy provider. No stdlib type-name workaround was added.

Compatibility: **artifact compiler9, semantic metadata4**, format2 and MVIR4.
Compiler1–8 artifacts reject before payload decoding. The canonical sysroot was
explicitly rebuilt. Reader checks reject invalid binder owners/type references
and inconsistent method contracts; this is not a complete type-graph audit.

Final focused verification on Windows GNU:

- [Semantic/artifact internals](evidence/r2-public-contract-internals-final.txt):
  **206 PASS**, cargo exit0.
- [CLI build](evidence/r2-public-contract-build-final.txt) and
  [canonical sysroot build](evidence/r2-public-contract-sysroot-final.txt): exit0,
  **49 provider artifacts**.
- [CLI regression matrix](evidence/r2-public-contract-cli-final.txt): **5 harness
  cases PASS**, cargo exit0. The new public-contract case checks 11 fingerprint
  comparisons (8 changed contracts, 3 invariance controls), 7 native executions,
  108 semantic check/build rejections, 16 stale-dependent artifact rejections,
  2 corrupted-metadata controls and direct decoder retention invariants. Source
  and relocated artifact graphs cover baseline, renamed and reversed providers;
  artifact-only graphs have no provider source. Method/generic/trust/execution
  dependency CLI regressions also pass.
- [Driver regression run](evidence/r2-public-contract-driver-final.txt):
  **45 PASS, 1 FAIL**, exit101. The failing DAG oracle expected 209 edges;
  slice's explicit copy dependency produces 210. The test now checks duplicate
  dependencies and acyclicity instead of freezing an incidental edge total.
  The [DAG rerun](evidence/r2-public-contract-dag-final.txt) is **6/6 PASS**, exit0.
  The initial driver invocation selected a nonexistent test target and ran no
  tests; its [command error](evidence/r2-public-contract-driver.txt) is retained.
- [Workspace compile](evidence/r2-public-contract-workspace-check.txt): exit0.

Intermediate evidence is retained without replacing failed verdicts: initial
CLI controls exposed body-local fingerprint leakage and reversed declaration
lookup failure. The expanded matrix exposed missing nominal-bound collection.
Later runs exposed the missing copy import and a field/declaration binder-owner
bug introduced during the metadata rewrite. Third/fourth logs contain transient
Rust editing errors before fixtures could execute. Corrected final results
close these reducers, not every public ABI/layout or generic well-formedness
domain. The last full workspace run remains **FAIL at de9977d** and predates
compiler8/9. Target identity, closure capture destruction, named/default calls,
broader generic/projection/layout domains and R3–R5 release gates remain open.

    Compiler Change
        Capability: Declaration-owned generic contracts, separate checked impl metadata, stable dependency fingerprints and forward impl indexing.
        Why stdlib exposed it: Independent method binders and generic provider validation require preserved element constraints across source/artifact compilation.
        Why it is generic: Declaration ownership and canonical type/trait shapes determine contracts; library type/provider names do not.
        User-defined type benefiting: Holder, Bounded, Tag, First/Second, Marker and independent method binders.
        Tests: interface_constraints_cli, method_resolution_cli, generic_typing_cli, artifact_execution_dependencies_cli, compiler_trust_diagnostics_cli_parity, semantic/reader and driver regressions.
        New intrinsic/lang_item?: NO
        Stdlib-specific branch?: NO

SKILL IMPACT: capability validation now explicitly requires public-constraint
fingerprints and stale-dependent rejection, with binder/order/body invariance
controls. Portable AST rechecking cannot substitute for public dependency
identity. No skill grants release/freeze authority.


## Configured target and actual object identity — 2026-10-05

The Windows-GNU manifest/object reducer is repaired. Provider manifests derive
format, pointer width and endianness from the same LLVM target machine used for
emission; ABI identity binds LLVM's default ABI/data layout. CPU/features are
retained and validated with strict equality. LLVM modules receive that triple
and layout before lowering. The artifact header now carries the manifest triple;
readers reject disagreement and noncanonical padding. Object parsing validates
relocatable format, architecture, width and byte order where represented.

Existing sidecars must equal the artifact's embedded object and match the target.
A stale/mutated sidecar rejects instead of supplying a different implementation.
Extraction failures propagate diagnostics. The public-bound stale-dependent
harness now removes a sidecar extracted from the preceding variant when replacing
the dependency artifact, so it reaches the intended interface mismatch gate.
The independent sidecar mismatch control remains a required rejection.

Compatibility: **compiler10**, format2, semantic metadata4, MVIR4. Compiler1–9
artifacts reject before payload decoding. Backend/driver share LLVM linkage
configuration; standalone backend tests now link without relying on driver
link flags. LLVM target-machine creation is shared by ordinary emission and
static-data layout rather than using two relocation configurations.

Final focused evidence:

- [Backend/artifact internals](evidence/r3-target-internals-final.txt): **17 PASS**,
  exit0; descriptor probes cover Windows COFF, Linux ELF, Darwin Mach-O and
  32-bit ELF. These are LLVM descriptor probes, not native target certification.
- [CLI matrix](evidence/r3-target-cli-final.txt): **4 harness cases PASS**, exit0:
  target identity, public constraints, execution dependencies and module-constant
  storage. Target controls execute 2 relocated source/artifact programs and
  reject 22 check/build invocations with E6001 for triple, CPU/features, format,
  ABI, byte order, pointer width, architecture, invalid object, header and sidecar.
- [Driver regressions](evidence/r3-target-driver-final.txt): **55 PASS**, exit0;
  metadata/parity, bound chain, canonical DAG, slice iterators, mangling, linking,
  function pointers, strict rejection and backend fail-closed cases.
- [CLI build](evidence/r3-target-build-final.txt),
  [official sysroot](evidence/r3-target-sysroot-final.txt) (**49 providers**) and
  [workspace compile](evidence/r3-target-check-final.txt): exit0.

An additional fail-closed guard rejects native lowering with non-64-bit pointers:
existing lowering still contains 64-bit word/layout assumptions. It is not safe
to accept that target merely because its descriptor is accurate. After this
narrow guard, [internals](evidence/r3-target-internals-guard-final.txt) pass
**18/18**, including a valid 32-bit descriptor whose lowering rejects; the
[target CLI rerun](evidence/r3-target-cli-guard-final.txt) passes on the latest
binary with another fresh sysroot. The larger native suites above precede this
32-bit-only guard; they were not relabeled as a new full run.

Retained intermediate FAIL evidence: standalone backend linking initially lacked
LLVM flags (no tests executed); format-only parsing initially accepted an object
with an unknown machine; the next header control rejected with the wrong typed
read code; and the public-bound harness first left an old object sidecar beside
a replacement artifact. These were corrected separately. A transient guard test
used a nonexistent Module::default constructor; its compiler error predates test
execution and is retained, not claimed as an original language defect.

This closes the Windows ELF/COFF metadata reducer and the tested object/sidecar
identity controls. It does not certify arbitrary target lowering/layout, the
advertised native runtime/compiler matrix, every object subtype/feature policy,
all ABI/public-layout domains, or release. The last full workspace verdict is
still FAIL at de9977d. Closure capture destruction, named/default calls, broader
generic/projection/layout obligations and R3–R5 remain open. Toolchain host is
Rust 1.98.0 **x86_64-pc-windows-msvc**; Luna emits **x86_64-pc-windows-gnu** through
LLVM18.1.8. These two target identities must not be conflated.

    Compiler Change
        Capability: Validate configured target/header/object/sidecar identity and reject incomplete 32-bit lowering.
        Why stdlib exposed it: Canonical provider artifacts exposed a Windows triple paired with a fabricated ELF label.
        Why it is generic: LLVM target/layout and object container identities apply to every provider; no stdlib type/provider names determine behavior.
        User-defined type benefiting: The ordinary api::answer provider plus relocated generic/constant/dependency providers.
        Tests: artifact_target_contract_cli, interface_constraints_cli, module_const_storage_cli, artifact_execution_dependencies_cli, backend/reader and 55 driver regressions.
        New intrinsic/lang_item?: NO
        Stdlib-specific branch?: NO

SKILL IMPACT: capability validation now distinguishes format labels from actual
header/object/sidecar identity and descriptor probes from native conformance.

## Full target-checkpoint regression — 2026-10-05

Source revision: `39a2a9bf7a9ed56539ccff7bbfa671aab3c952b5`.
`cargo test --workspace --no-run` succeeded, then
`cargo test --workspace --no-fail-fast` exited 101:
**1,262 passed, 34 failed, 1 ignored**, nine failing targets.
The [summary](evidence/workspace-target-summary.json),
[start](evidence/workspace-target-start.json), [completion](evidence/workspace-target-exit.json)
and [lossless raw log](evidence/workspace-target.txt.gz) pin command, times,
source revision, CLI/runtime hashes and all 34 panic details. CLI/runtime hashes
were unchanged across the run. Rust host was Windows MSVC; Luna target Windows GNU.

Failure triage:

- 31 failures involve mismatched object sidecars. The I/O suite rebuilds the
  shared canonical sysroot concurrently with its own readers. The String parity
  test replaces the shared `string.llib` with a newly built artifact while
  retaining the previous `string.obj`, contaminating later suites. Artifact
  rejection is correct; test isolation and actual parity coverage require repair.
- The struct-lifetime compatibility test expects semantic metadata v3, while
  declaration-owned generic contracts adopted v4. Its explicit version oracle
  needs reconciliation, preserving incompatible-version rejection.
- The contract-bootstrap test writes Iterator/IntoIterator bounds without the
  declared trait arguments. Strict bound checking rejects them; the valid
  visibility control needs complete trait applications and useful failure output.
- The generic-drop CLI still fails source and artifact closure-capture cleanup,
  exit 3. This compiler correctness gap remains open.

Independent artifact-only reducers on the same source revision establish two
additional correctness defects: reordering public struct fields, or reordering
an enum's variants, leaves the interface fingerprint unchanged. Replacing a
valid dependency bundle then permits an old wrapper to run with the changed
layout/discriminants: baseline exit 0, changed dependency exit 6 (13 instead of
7). These are separate silent wrong-code findings, not covered by the workspace
failure count. Ordered nominal representation must enter dependency identity.
No freeze, release, tag or merge readiness is claimed.

### Isolated regression repair

The test-only repair keeps object-sidecar validation intact. The I/O builder
now owns a copied sysroot. String parity builds into its own sysroot with a
matching object, removes String source from the artifact mode, and executes a
separate source mode without String artifacts. Bootstrap bounds supply the
actual Iterator/IntoIterator arguments; schema expectations track v4.
After a fresh canonical build, the [eight-suite rerun](evidence/r0-isolation-focused.txt)
passes **96/96**, exit 0; [sysroot rebuild](evidence/r0-isolation-sysroot.txt) exits 0.
These focused results close those exact failures only. The full 39a2a9b run,
closure drop failure and nominal representation reducers remain recorded FAIL.

## Ordered nominal representation checkpoint — 2026-10-05

Intended contract: dependency identity protects layouts/discriminants required
by consumer code, even when a type in a public signature is private. Names and
nominal IDs alone do not certify ABI compatibility.

The [pre-repair struct](evidence/nominal-layout-before-struct.json) and
[enum](evidence/nominal-layout-before-enum.json) reducers record matching
fingerprints despite changed representation: stale consumers execute exit 6
instead of baseline 0. The [private-type probe](evidence/nominal-layout-before-private.json)
records acceptance of an old wrapper after changing the hidden type; the new
acceptance fixture separately verifies rebuilt size 8 versus original size 16.

Compiler11 / semantic metadata5 now fingerprint a separate ordered nominal
representation table. Traversal includes public contracts, owned impl contracts
and transitive private nominal types; foreign layouts belong to dependency
identity. Recursive references remain nominal rather than recursively expanding
layout in type identity. Unreachable private declarations are excluded.
Public struct fields and enum discriminants recover their order in metadata;
reader validation rejects missing definitions, bad indices, duplicate members,
wrong kind/owner and disagreement with exported field contracts.

Permanent CLI fixtures are in `tests/luna/language/nominal_layout`, orchestrated
by `nominal_layout_cli`: nine native executions across source, relocated
artifact-only and rebuilt consumers; six typed stale-dependency rejections;
representation corruption/decoder checks; recursive hidden-type reachability
and binder/body/unreachable-private identity controls. Matching sidecars ensure
stale tests reach dependency identity rather than object mismatch rejection.

[Five CLI regression suites](evidence/r2-layout-cli-regressions.txt) pass:
nominal layout, generic constraints, target identity, execution dependencies and
module constants. [Internal/driver regression](evidence/r2-layout-internal-regressions.txt)
passes **82/82**. [Canonical rebuild](evidence/r2-layout-sysroot.txt) and
[workspace check](evidence/r2-layout-check.txt) succeed. The
[checkpoint record](evidence/nominal-layout-checkpoint.json) pins binary/runtime
and raw evidence hashes. Intermediate compile errors and mistyped test-target
invocations remain in the evidence directory; they are not counted as executed
test failures or rewritten into passing logs.

Scope remains bounded: canonical artifacts require portable AST. Legacy
metadata-only layout reconstruction retains earlier limitations. Broader ABI,
malformed type-graph validation, closure cleanup, named/default calls and R5
full-candidate gates remain open. Last full workspace verdict remains FAIL.

Compiler Change
- Capability: canonical dependency identity for ordered, ABI-reachable nominal representations.
- Why it is generic: one representation traversal covers arbitrary user-defined structs/enums and hidden types, independently of stdlib names.
- User-defined beneficiaries/tests: Record, Color, Hidden/Visible and native stale-dependent probes above.
- New intrinsic/lang_item: NO. Stdlib-specific branch: NO.

SKILL IMPACT: REFINE capability validation with nominal ABI/reachability controls;
REFINE testing strategy with isolated artifact mutation and genuine parity.
Both changed skills were reread and related guidance checked for conflicts.

### Remaining shared sysroot mutation audit

Five additional legacy parity tests (Box, collection iterators, HashMap,
HashSet and iterator collection) still copied newly built artifacts over shared
canonical libraries. They now use isolated source/artifact provider pairs,
matching sidecars and explicit route exclusions. Four executable comparisons
run both routes and compare the complete result; the existing Box metadata test
checks admission in both routes. JSON publication in the sysroot-invariants
suite is isolated too, and checks the emitted manifest/object/metadata.

[Five existing suites](evidence/r0-parity-isolation.txt) pass **66/66**;
[sysroot invariants](evidence/r0-json-isolation.txt) pass **6/6**. No shared
canonical publication remains in these parity tests. This is test-harness
repair, not new language support or a full-workspace PASS.

## Module constant lexical scope repair — 2026-10-05

The [before CLI probe](evidence/r1-module-private-before.txt) rejects valid private
module constant construction, resolving a same-named root struct instead of
the module-owned struct. Early evaluation iterates a flat topological list and
had lost each declaration's lexical scope. Type annotation, initializer and
comptime checking now use the constant symbol's defining scope and restore the
previous scope after evaluation. No visibility/ownership relaxation is involved.

Permanent fixtures `module_const_storage/private_provider.ln` and
`private_scopes.ln` cover two modules with same-named private types/constants,
an outer shadow, forward dependencies and root-scope restoration. The
[expanded CLI rerun](evidence/r1-module-private-final.txt) passes eight native
source/relocated-artifact executions and the existing twenty typed escape,
mutability, privacy and VM-pointer rejection controls. An
[earlier focused rerun](evidence/r1-module-private-first-rerun.txt) is preserved
separately. [Semantic tests](evidence/r1-module-private-semantic.txt) pass
191/191. This fixes module-constant scope; it does not implement named/default
arguments. Their definition-site evaluation still needs its own call-binding,
ownership, generic and provider acceptance matrix.
