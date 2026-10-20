# R4 closing record — 2026-10-14

**TASK:** R4 Release Documentation & Ledger.
**VERDICT:** CLOSED / VERIFIED IN DOCUMENTATION AND EVIDENCE SCOPE.
**RELEASE:** BLOCKED / NOT VERIFIED. **R5:** PENDING; not started.

Base: `3f8646c654b99b67bae57ee290b7a7e23d717b40`.
Branch: `r4-release-docs-ledger`.
Documentation implementation/verification commit:
`b03fa63f5027491c90fc9070e3229a583d875669`.
Language implementation tested by E-wave:
`e4f9fb530d72628c81dfa29d117afeb99cc7664c`.
Protocol remains **25 / 9 / 5 / 2**.

The closure commit containing this record adds the final report, verification
evidence, generated inventory, closure status entry points, and a Windows
long-path note in the R5 checklist.
Its exact SHA is reported after committing and is not embedded into itself.
On verified PASS, the candidate is fast-forwarded from the base to that closure.

## Required results

| Closing requirement | Result / evidence |
| --- | --- |
| Exact release ledger | PASS — [25 wave/follow-up records](release-ledger.json), full closure/decision and integrated repair SHAs |
| Wave reconciliation | PASS — [A/C/D/E plus follow-ups](README.md); old C1/A1 source aliases and failed acquisitions preserved as history |
| Diagnostic conformance inventory | PASS as documentation inventory — [canonical DIAG-1..10 map](diagnostic-inventory.md); full structural conformance remains PARTIAL and required in R5 |
| Target qualification matrix | PASS as inventory — [Windows tested scope; Ubuntu normal/ASan NOT VERIFIED](target-matrix.md) |
| Accepted deferrals and known limitations | PASS — [precision/presentation decisions, comptime/VM bounds and remaining audit scope](known-limitations.md) |
| Executable R5 checklist | PASS — [fresh checkout/build/ABI/focused/parity/docs/workspace/CI/ASan commands and missing audits](r5-checklist.md); current Cargo test target paths checked; no R5 execution |
| Evidence paths and SHA reconciliation | PASS — 108 distinct structured evidence files; 1004 preexisting immutable audit files protected; lossless 24-row legacy migration; repository and acquired-byte hashes distinguished |
| Docs validator | PASS — [saved verification](evidence/r4-verification.json); every current Markdown/HTML evidence link checked |
| Implementation/runtime/test changes | NONE — tree equals E-wave tested implementation; no full workspace reruns during R4 |
| No release-ready claim | PASS — release BLOCKED; Ubuntu GNU normal/ASan, complete diagnostics and recorded comptime/VM limits remain uncertified |

## Verification boundary

[Saved results](evidence/r4-verification.json) distinguish the first documentation
commit, local closing-document checks and the separate LF checkout. Both
`validate_r4_ledger.py --require-unchanged-implementation` and
`validate_v01_docs.py` pass. The clean LF checkout validates only documentation
and provenance; it does not certify Ubuntu, run compiler acceptance, or start R5.

The evidence migration preserves original historical fields, including nullable
or noncanonical introducing pins. Every current closure in the release ledger
is in the base ancestry. The three final6 log hashes and 236 arithmetic records
are read and reconciled, not rerun. Three historical scratch references now have
durable lossless archives. Git text normalization does not overwrite acquired
bytes: differing text has an explicit original-byte GZ archive.

All existing saved logs, JSON/GZ and compatibility artifacts protected by the
manifest remain unchanged. Current status documents receive dated reconciliation
without replacing historical failures. The original repair status.json is
unchanged. Nine retained E1 logs, the referenced E1 sysroot and user modification
`.commandcode/taste/taste.md` remain untouched.

The LF docs checkout initially exposed Windows filename-length failures in a
deep output path. Restoring the missing files after enabling local Git
`core.longpaths` produced a clean checkout and passing docs checks. The R5
Windows checklist records that setup requirement and still requires clean
status independently of a checkout exit code.

No main merge or release tag is performed. The candidate fast-forward is the
only integration action authorized on R4 PASS. R5 requires a separate task and
actual exact-candidate target/diagnostic qualification before release readiness
can be considered.
