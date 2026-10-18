# Completion evidence matrix

Current verdict: **NOT COMPLETE**. This matrix derives from the original
[R5 handoff](../0.1-alpha-completion-2026-10-04/r4/r5-checklist.md),
[completion plan](../0.1-alpha-completion-2026-10-04/README.md), and
[draft maintainer handoff](../r5-missing-return-2026-10-17/RELEASE-HANDOFF.md).
It does not replace or narrow those requirements.

Selected implementation: `3a72f9464fa921410b63551f7eac03414cf51e2b`.
Historical results from other pins cannot fill the pending cells below.

| Requirement | Evidence required at selected pin | Current evidence / verdict |
| --- | --- | --- |
| R5-PIN | Fresh remote detached clone, exact HEAD, clean initial/final worktree and index, tools/target/source identities | Ubuntu initial identity and protected implementation trees verified; normal final status has two generated documentation-index changes; Windows normal wrapper enforces clean final status; both full-workspace clean-final results pending |
| R5-BUILD | Debug/release CLI, native runtime ABI, all 49 source providers rebuilt, canonical headers/object identity, versions 25/9/5/2 | Both native normal lanes verified runtime, debug/release CLI and all 49 canonical native provider bundles |
| R5-FOCUSED | Complete named ownership/drop/async/casts/raw-view/calls/traits/comptime/provider/module/retained-contract suites; source/artifact parity and rejection identity | Both native normal lanes PASS; Windows has 12 complete batches |
| DIAG-1..10 | Producer/code/owner/span/related-label/event-origin map; per-code behavioral coverage and declared exceptions; poison/determinism tests; registry and inventory drift gates | Both normal lanes supply diagnostic/per-code/span/registry/inventory logs; final complete producer/owner/related-label source review remains required; no universal claim from registry parity alone |
| Diagnostic debt closure | Typed E4002/E4003; step-limit/escape behavioral assertions; run preserves originating compile code; inventory debt empty | Both normal lanes diagnostic suites, typed-code/span policy and inventory drift gates PASS |
| E3004 | Reachable fallthrough rejects with anchored E3004 during check/build before executable publication; valid explicit/tail/endless-loop controls; source/fresh artifact paths | Both normal lanes missing_return_cli PASS across source and artifact modes |
| Linux native math linkage | Independent user FFI fixture plus the five unchanged failures across the three original driver suites | Ubuntu normal PASS: user FFI source/artifact plus all five unchanged regressions (19 tests across three suites); Windows normal native_math_link_cli PASS |
| Arithmetic and bounds traps | Checked/wrapping arithmetic native signatures L003/L005; strict A5 L002 with actual index 3 / length 2; failure cannot mean any native crash | Both normal arithmetic gates PASS; hosted Windows strict L002 replay **4/4 PASS**, verified native stderr signature and actual index/length |
| R5-COMPTIME-LIMITS | Valid branch/dependency reducer, explicit withdrawn-probe characterization, 64-bit GNU VM/runtime/static layout parity; unsupported statement execution rejects | Ubuntu **PASS in executed scope**: [88 PASS / 0 FAIL / 8 characterizations](evidence/ubuntu-comptime-vm-verification.json); Windows also **88 PASS / 0 FAIL / 8 characterizations**, verified in native normal archive |
| R5-TARGET-WINDOWS | Native GNU ABI, Whole-File I/O, formatting/Unicode, artifact target, CLI/provider parity, matching job/SHA | Windows native normal PASS at the selected pin; complete workspace separately pending |
| R5-TARGET-UBUNTU | Native GNU ABI, Whole-File I/O, formatting/Unicode, canonical Linux target, CLI/provider parity, matching job/SHA | Ubuntu normal **48/48 gate PASS**, including canonical target and native execution; workspace separately pending |
| R5-ASAN-UBUNTU | libasan instrumentation, partial-read cleanup executes, RuntimeAbiTests exit 0, no sanitizer/leak report, scope documented | **PASS in C runtime scope**: [verified 19 gates / 42 files](evidence/ubuntu-asan-verification.json); raw logs confirm libasan, partial-read test and 100% CTest pass |
| R5-DOCS | Generation/validator, all 16 CLI example contract expectations, legacy foreach characterization, native README quickstart | Both native normal docs/example/native quickstart gates PASS (each 14 contract rows + 2 legacy characterizations); final edited public docs consistency separately verified |
| R5-FULL Windows | Completed no-fail-fast triage, then default-parallel final; zero failures; named ignored cases; logs/exits/hashes | Triage and final both verified: **1414 PASS / 0 FAIL / 1 IGNORED** |
| R5-FULL Ubuntu | Completed no-fail-fast triage, then default-parallel final; zero failures; named ignored cases; logs/exits/hashes | Triage and final both verified: **1414 PASS / 0 FAIL / 1 IGNORED** |
| R5-REVIEW | Every requirement traced to pin/command/target/mode/result/evidence; no unresolved critical/high defect in adopted scope; ledger/public status/target matrix/limitations agree | All terminal artifacts verified; final review found a confirmed nested move-origin gap (DIAG-3/9), so release remains blocked |
| Candidate advancement | Fast-forward only to the selected qualified implementation after all requirements pass | Not performed; candidate c085449b |
| Maintainer release decision | Concrete completed handoff with final pin, results, links, limitations and proposed release contents | Draft only; tag/main merge/PR/publication remain maintainer actions |

Artifact verification must preserve the downloaded ZIP digest, original manifest
and compressed/raw log hashes. A job success is insufficient when its scope
omits the requirement. Every failure or interruption remains historical evidence;
no timeout, missing DLL, signal or unrelated crash can be reclassified as a pass.
