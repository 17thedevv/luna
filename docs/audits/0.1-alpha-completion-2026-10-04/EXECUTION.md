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
