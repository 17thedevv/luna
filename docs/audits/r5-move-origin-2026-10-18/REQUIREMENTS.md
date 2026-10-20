# Luna 0.1 replacement qualification requirements — closure

**Historical pin only.** The Windows package portability failure superseded
this release verdict. The requirements below remain mandatory; their current
replacement evidence at `b5cb77c3` is tracked in the
[CPU-baseline qualification](../r5-cpu-baseline-2026-10-20/README.md).

**2026-10-20: READY FOR RELEASE REVIEW.** Tested implementation and candidate:
`8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5`.
The [original checklist](../0.1-alpha-completion-2026-10-04/r4/r5-checklist.md),
[completion scope](../r5-linux-native-link-2026-10-18/REQUIREMENTS.md) and
[remaining plan](REMAINING-PLAN.md) retain their requirements.

| Requirement | Implementation/regression and exact-pin evidence | Verdict |
| --- | --- | --- |
| R5-PIN | All eight ZIPs: fresh remote clone/detach, exact pin, clean initial/final state, recorded tools and source-tree objects matched against Git | PASS |
| R5-BUILD | Both native hosts: runtime ABI, debug/release CLI, official 49-provider rebuild; versions 25/9/5/2; canonical native object/header identities | PASS |
| R5-FOCUSED | Ownership/drop/async, casts/views, calls/defaults, traits/comptime, provider/module/retained contracts, arithmetic/grammar, formatting/Unicode, artifact target, registry and I/O named gates; source/fresh-artifact controls preserved | PASS |
| DIAG-1 | Typed CLI/Driver and comptime identities; run retains original compile code; per-code coverage and compiler trust regression | PASS IN REVIEWED SCOPE |
| DIAG-2/3 | Real declaration/composite-expression anchors; previous-definition, impl/candidate, import and language-item locations; source-anchor policy and structural labels. Declaration-location suite passes all four complete workspaces and local focused acquisition | PASS IN REVIEWED SCOPE |
| DIAG-4/6 | Typed source-event identity; structural cause union; public source ordering; identical-wording distinct events and independent multi-function CLI processes; common identity/idempotence regressions | PASS IN REVIEWED SCOPE |
| DIAG-5 | Error-type poison consumers and visible-root cascade suppression; cast_poison_cli, comptime_diagnostic_cascade_cli, diagnostic_conformance_cli | PASS IN REVIEWED SCOPE |
| DIAG-7/8/10 | 72-code identity/span policy, typed phase ownership and exact registry parity; inventory64 produced/8 reserved/4 declared fixture limits/debt 0 with reasons | PASS WITH DECLARED LIMITS |
| DIAG-9/internal dataflow | Nested/equal-state/loop joins, origin-only convergence, predecessor permutations, reinitialization and compacted-field causes; check/build and source/artifact negatives plus native controls | PASS |
| E3004 | Missing-return negatives reject before executable publication with spanned E3004; valid return/loop controls execute in both provider modes | PASS; earlier no-producer claim RETRACTED |
| Native Linux math linkage | Generic linker suffix plus independent FFI parity; five unchanged regressions across three driver suites (19 tests) | PASS |
| Arithmetic/bounds | Checked/wrapping signature assertions L003/L005; hosted strict bounds debug/release x source/artifact, L002 index 3 length 2 in every stderr | PASS |
| R5-COMPTIME-LIMITS | Each host88 PASS/0 FAIL/8 characterized; layout/VM/static/runtime controls; standalone statement rejection; characterization kept separate | PASS IN NAMED GNU64 SCOPE |
| R5-TARGET-WINDOWS | Hosted normal plus triage/final; GNU LLIB/COFF 0x8664; native ABI/I/O/format/Unicode/parity | PASS |
| R5-TARGET-UBUNTU | Native canonical x86_64-unknown-linux-gnu/ELF64; normal48 gate records plus triage/final | PASS |
| R5-ASAN-UBUNTU | 19 gates; libasan.so.8; partial-read cleanup executes; RuntimeAbiTests 100%; no sanitizer/leak report | PASS IN C RUNTIME SCOPE |
| R5-DOCS | Native example gates:14 contract rows plus2 legacy characterizations; docs validator and native quickstart; current public docs/site regenerated separately | PASS |
| R5-FULL Windows | Triage --no-fail-fast then default-parallel final; each 236 groups,1421 passed/0 failed/1 ignored, exit 0 | PASS |
| R5-FULL Ubuntu | Triage --no-fail-fast then default-parallel final; each 236 groups,1421/0/1, exit 0 | PASS |
| R5-REVIEW | [Source/evidence reconciliation](evidence/pin-8525067f/closure-review.json); [eight original ZIPs, metadata and independent verification](evidence/pin-8525067f/all-artifact-verification.json); no unresolved finding in reviewed adopted scope | COMPLETE IN TESTED SCOPE |
| Candidate advancement | Remote candidate fast-forwarded c085449b→8525067f without force; main unchanged | DONE |
| Maintainer handoff | [Concrete proposal](RELEASE-HANDOFF.md), [report](REPORT-R5-QUALIFICATION-8525067f.md), [targets](TARGET-MATRIX.md), [limits](LIMITATIONS.md), [ledger](evidence/pin-8525067f/lane-state.json) | READY FOR REVIEW |
| Full release | Maintainer PR/main/tag/publication; packages from verified release commit, clean Windows/Ubuntu package smoke and final asset/tag reconciliation | PENDING; goal remains active |

Ignored test in each workspace:
`mutable_references::sem_mutref_06_invalid_fixture`.
Original failures, superseded pins and local interruptions remain historical.
No prior-pin PASS, arbitrary crash, timeout or missing DLL is accepted.
No new semantic freeze, target expansion or blanket API certification follows.

Hosted focused diagnostics includes per-code, missing-return and move-origin
suites. It does not separately select declaration-location; that suite runs
in all complete workspaces. This corrects earlier wording about every focused batch.
