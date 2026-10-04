# Execution ledger — Luna 0.1 completion

Baseline implementation: `c8d0559500e78ef85c96ac842e824e639917cbad`.
This supplements the dated [plan](README.md); no full release PASS is claimed.

Latest implementation checkpoint: `f5936c3344788168320dfd4b661532c2cfad8a8d`
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
open. A separate maintainer question now distinguishes module-const static
storage from temporary value materialization; neither interpretation has been
silently adopted to make the borrowed Option/Result parity test pass.

SKILL IMPACT: none. Existing artifact guidance already requires compiler
identity validation and fail-closed rejection without rebuild/fallback.
