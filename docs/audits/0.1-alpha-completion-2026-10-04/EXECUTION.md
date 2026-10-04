# Execution ledger — Luna 0.1 completion

Baseline implementation: `c8d0559500e78ef85c96ac842e824e639917cbad`.
This supplements the dated [plan](README.md); no full release PASS is claimed.

Implementation checkpoint: `7f818610428da3b39565f2f8a88386b0dfc4707c`
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
