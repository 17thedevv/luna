# E-wave qualification: RangeInclusive terminal contract decision

**Verdict: BLOCKED on a public stdlib decision; E-wave is not CLOSED.**
Latest tested production/test source pin:
`939d53e0ad7719c7c4a3eb4b915173fedeb1a129`.
Candidate/E2 remains `6017ba8d8db40cf0bc16498f639e5d830c0e827c`.
Versions: compiler24 / metadata9 / MVIR5 / format2. No push/fast-forward,
main merge, or release tag is justified by this checkpoint.

## Verified scope and failed final gate

- Four raw 59-row matrices PASS (236 acquisitions), exact L003/L005 panic
  signatures, source/fresh-artifact and compiler debug/release profiles.
- Original 46-row reconciliation: 22 TRAP / 8 PASS / 12 WRAP / 4 COMPILE_ERROR;
  each comptime invocation has exactly one E4005.
- Diagnostic identity acquisition PASS in both profiles. In the full run,
  ownership, E1 four-mode qualification, real22/23 rejection, fresh24 acceptance,
  relocation, wrapping, comptime cascade, trust, interface constraints and E2
  grammar suites PASS. HashMap14 / HashMap-storage11 / HashSet8 PASS.
- Full `cargo test --workspace`, default parallelism, stops at **138 binaries,
  877 passed / 1 failed / 1 ignored / exit101**. Three consecutive full passes
  have not been achieved.

[Full failed acquisition](e1-workspace-final4-2026-10-13.json) and lossless
compressed log are immutable. It fails `test_range_and_inclusive_boundaries`
in `stdlib_phase4b_iter_range_tests`. At `254u8..=255u8`, RangeInclusive marks
`exhausted`, then uses `step_forward` to obtain a replacement for the terminal
owned value. Ordinary addition at MAX correctly traps L005. First divergence is
the unnecessary terminal advance, not a compiler overflow-checking defect.

An unrelated test writes the tracked debug output `test_model.mvir`, causing
the acquisition's `code_diff_empty_after` to be false. The writer is
`sem_gap_19_alias_safety_audit::test_place_model_investigation` (emit_mvir=true,
relative filename, no output_path). This known generated dump was restored to
the pinned contents after the run. Its output path is now isolated under
TempWorkspace; focused suite **9 PASS**, with no tracked dump modification.
This change is a test-isolation repair, not an arithmetic or library contract
change. The failed raw record is not rewritten into PASS.

## Decision and concrete feasibility

[Decision proposal](../../../proposals/e1-range-inclusive-terminal-slot-2026-10-13.md)
documents two choices:

1. **Recommended:** public `current: T` becomes `current: Option<T>`. Extract
   the last owned value by replacing Some with None. Step remains checked,
   generic non-Copy values need no Clone/Default, but direct field/struct-literal
   source compatibility and nominal ABI/interface fingerprint change.
2. Preserve fields and explicitly adopt modular integer Step.forward(MAX) via
   wrapping_add. This decides a public boundary policy and does not give custom
   Step types a general terminal extraction guarantee.

The isolated ordinary-Luna prototype for choice1 compiles and runs exit0 in
all four profile/provider combinations. It tests u8 MAX and a non-Copy Counter
at i32 MAX, with exactly three observable drops and no final step_forward call.
It is **feasibility evidence on a user type**, not canonical-provider completion.
Production `iter_adapters.ln`, Step, and RangeInclusive remain unchanged while
the maintainer decision is pending. No compiler-specific Range branch or new
intrinsic/ABI/wire operation is proposed.

Initial borrowed-enum-pattern and incorrect-provider-name prototype failures,
plus a missing LLVM DLL PATH acquisition, remain development failures rather
than being counted as passes. The working prototype uses the existing
Option.as_ref and mem::replace APIs. Full canonical-provider/drop/edge-case
qualification follows a decision.

## Continuation

Maintainer continuation, 2026-10-13: **A approved — Option slot, keep Step
checked**. The public current field amendment is adopted in the
[stdlib contract](../../../spec/0.1/stdlib.md). The historical failed run and
unchanged-production observations above describe the pre-approval checkpoint;
they do not describe the subsequent implementation or certify its final gates.

Implement the selected policy, run focused native source/artifact checks for
all supported integer limits, empty/inverted/repeated exhaustion and non-Copy
drops, regenerate the sysroot and verify stale-interface rejection. Pin the
result, reacquire current matrices and restart three consecutive full workspace
passes. Then docs validator0, closure report, implementation push and candidate
fast-forward may proceed. Stop before R4/R5 tagging or main merge.

Ubuntu GNU normal/ASan and broader DIAG-1..10 conformance remain NOT VERIFIED
R5 obligations. Standalone comptime statement execution remains the separate
approved E4005 support gap. E2 stays CLOSED in its tested scope.
