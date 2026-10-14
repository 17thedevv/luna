# R4 — Release documentation and ledger

Date: 2026-10-14. Branch: `r4-release-docs-ledger`.
Base/candidate on entry: `3f8646c654b99b67bae57ee290b7a7e23d717b40`.
Tested implementation: `e4f9fb530d72628c81dfa29d117afeb99cc7664c`.
Protocol: **compiler25 / metadata9 / MVIR5 / format2**.

**Release: BLOCKED / NOT VERIFIED. R5: PENDING.** R4 reconciles existing
evidence and current documentation. It grants no new capability or release
certification and does not run R5. A/C/D/E closures retain their tested scopes.

## Records

- [Machine-readable exact ledger](release-ledger.json) and [schema](release-ledger.schema.json).
- [Diagnostic inventory](diagnostic-inventory.md), using the canonical DIAG numbers.
- [Target matrix](target-matrix.md) and [known limitations / deferrals](known-limitations.md).
- [Executable R5 handoff checklist](r5-checklist.md).
- [Legacy snapshot](legacy-status.snapshot.json) and [lossless migration](legacy-status.migrated.json).

## Wave and follow-up reconciliation

Every SHA below resolves to the canonical integrated history and is an ancestor
of the R4 base. Introducing repairs are separate from closure/report commits.
Current regression evidence is the three final6 runs at the tested implementation;
the JSON names the suites observed in each raw log. Historical reports preserve
their earlier pins and failures; some old A1/C1 hashes are from a different Git
history and must not replace the canonical integration identities below.

| Item | Status / scope | Exact closure or decision SHA | Report |
| --- | --- | --- | --- |
| A1 — Closure environment destruction and consume-on-call | CLOSED / tested scope | `060aabb3897319e5f400da405470973c601a0871` | [record](../evidence/REPORT-A1-CLOSURE-ENV-2026-10-06.md) |
| A2 — Async suspended cleanup | CLOSED / tested scope | `f1e53f39602d7fc9e55229b386e7c3ec54b4c90f` | [record](../evidence/REPORT-A2-ASYNC-SUSPENSION-2026-10-07.md) |
| A2-FU1 — Shared DropFlagPlan / DropFlagTransition | CLOSED / tested scope | `d4d980fb300ab9871674eefe36c038db4574e21b` | [record](../evidence/REPORT-A2-FU1-DROP-FLAG-PLAN-2026-10-09.md) |
| A2-FU2 — CLI async source/fresh-artifact cleanup matrix | CLOSED / tested scope | `47f402ea7057eb25740ebe7350e56a4e7d7e4d03` | [record](../evidence/REPORT-A2-FU2-CLI-MATRIX-2026-10-07.md) |
| A3 — Remaining constant-index array elements after partial move | CLOSED / tested scope | `c3e55f4f06631eac52187bd49e709d57bfe3308f` | [record](../evidence/REPORT-A3-PARTIAL-AGGREGATE-CLEANUP-2026-10-07.md) |
| A4 — Cast rejection and poison containment | CLOSED / tested scope | `7aee152852e10f5d25286c114d384a2e297e3083` | [record](../evidence/REPORT-A4-CAST-POISON-CONTAINMENT-2026-10-07.md) |
| A5 — Raw slice/view loan transport | CLOSED / tested scope | `54df3f4300d19f6a7e9921879342df062abafe50` | [record](../evidence/REPORT-A5-RAW-SLICE-VIEW-CONTRACT-2026-10-07.md) |
| A6 — Unary logical NOT on bool | CLOSED / tested scope | `5939105d31434aee419b7dc36a15bd1f44c19d1f` | [record](../evidence/REPORT-A6-UNARY-LOGICAL-NOT-2026-10-07.md) |
| C1 — Named/default calls and comptime admission reconciliation | CLOSED / tested scope | `52c96ed705b156752c7918682f138442283205ef` | [record](../evidence/REPORT-C1-RECONCILE-2026-10-06.md) |
| C2 — Per-impl-header metadata and interface identity | CLOSED / tested scope | `22c33366a11f1d0b1331057a884b37e69bad42ed` | [record](../evidence/REPORT-C2-PER-IMPL-HEADER-METADATA-2026-10-08.md) |
| C3 — Projection fallback and proof depth | CLOSED / tested scope | `bf265798ef161f5ec340ec797f88f2cf085d4f75` | [record](../evidence/REPORT-C3-PROJECTION-FALLBACK-2026-10-08.md) |
| C4 — Comptime execution dependencies, bounded static precision | CLOSED / tested scope | `5994f5a3f90d753dd79c9c139fe2689e543d3710` | [record](../evidence/REPORT-C4-COMPTIME-DEPENDENCY-2026-10-08.md) |
| C5 — Artifact freshness classification | CLOSED / tested scope | `7b3c51855eb83b3b7bfff1d8f6d3c1a5f7b0f2e8` | [record](../evidence/REPORT-C5-ARTIFACT-FRESHNESS-CLASSIFICATION-2026-10-08.md) |
| D1 — Diagnostic registry and selected conformance reducers | CLOSED / tested scope; complete structural audit still R5 | `39004d69de418a81d74a2748a95a41d093ebc3bb` | [record](../evidence/REPORT-D1-DIAGNOSTIC-CONFORMANCE-2026-10-08.md) |
| D2 — Expression span precision | CLOSED / tested scope | `35260b6bc299703d51c5242ffaadc220c1563211` | [record](../evidence/REPORT-D2-EXPRESSION-SPANS-2026-10-08.md) |
| D3 — Tier-2/3 provider graph and rejection matrix | CLOSED / tested scope | `70002d84b641b94c3aa0755983fff618e0bb6121` | [record](../evidence/REPORT-D3-PROVIDER-TIER23-MATRIX-2026-10-08.md) |
| D4 — Retained language contracts | CLOSED / tested scope | `4024abc6beede7b4bb0fb83f2d12a83ea14df39d` | [record](../evidence/REPORT-D4-RETAINED-CONTRACTS-2026-10-08.md) |
| FIND-ASYNC-PROVIDER-01 — Real async provider HeapFree wire decoding | CLOSED / tested scope | `f4b7c0de177cb24155777f7dfe9b2727a34827b7` | [record](../evidence/REPORT-FIND-ASYNC-PROVIDER-01-2026-10-09.md) |
| D4-FU1 — Reject link on program entry; original leading-item observation retracted | CLOSED / tested scope | `5122d3ce73268eb2e6021e79005be9f70c316b76` | [record](../evidence/REPORT-D4-FU1-LINK-ENTRY-2026-10-09.md) |
| D1-FU2 — Suppress comptime cascade only with visible root | CLOSED / tested scope | `d7129cd14beac6a9906ecd29d1704f8f91004e97` | [record](../evidence/REPORT-D1-FU2-COMPTIME-DIAGNOSTIC-CASCADE-2026-10-09.md) |
| R5-HARNESS-01 — Owned temporary state, isolation, bounded residual cleanup | CLOSED / tested scope | `ccc52c723ed799f17350d7e110fab72505b147c0` | [record](../evidence/REPORT-R5-HARNESS-01-TEMP-SYSROOT-ISOLATION-2026-10-09.md) |
| E2 — Grammar/spec reconciliation without grammar expansion | CLOSED / tested scope | `6017ba8d8db40cf0bc16498f639e5d830c0e827c` | [record](../evidence/REPORT-E2-GRAMMAR-SPEC-RECONCILIATION-2026-10-11.md) |
| E1 — Checked arithmetic, provider admission/parity, terminal extraction and generic alignment | CLOSED / tested scope | `3f8646c654b99b67bae57ee290b7a7e23d717b40` | [record](../evidence/REPORT-EWAVE-CLOSURE-2026-10-13.md) |
| C4-FU1 — Branch-sensitive comptime dependency precision | DEFERRED, non-blocking (0.2) | `98b55952b5f51188318e8f447ab9d204355fc722` | [record](../MICRO-BACKLOG-2026-10-06.md) |
| D1-FU1 — E3001 presentation exposes internal value name | DEFERRED, non-blocking (0.2) | `98b55952b5f51188318e8f447ab9d204355fc722` | [record](../MICRO-BACKLOG-2026-10-06.md) |

A-wave is CLOSED. C-wave is CLOSED in tested scope. D-wave is CLOSED with
follow-ups classified; D1 closure is not universal DIAG-1..10 certification.
E2 is CLOSED and E1 is CLOSED / QUALIFIED IN TESTED SCOPE at the E-wave closure.
The C1 report's comparable failing workspace and older D3/D4 open findings are
historical: A1 and FIND-ASYNC-PROVIDER-01 have separate later closures above.
The original D4 leading-attribute parser observation was retracted; the actual
entry-link placement requirement was repaired and its phase ownership corrected
again during E1. No historical failure log is rewritten.

Cleanup checkpoint 2026-10-14 is accepted CLOSED / VERIFIED IN REPORTED SCOPE:
254 root files removed without changing refs, tracked evidence or implementation.
Nine logs and `.e1-fp-b99152562efa448c8ecf6d0573a3feeb/` remain protected.
The user modification `.commandcode/taste/taste.md` is outside this task.

## Ledger schema and migration

`contract_status` is independent of `implementation_status` and release status.
`implementation_commit` is nullable; `implementation_commits` preserves a repair
chain rather than assigning its work to a report commit. Null on a coverage-only
row means the introducing implementation is not claimed. `verified_at_commit`
identifies the tested source, not the commit storing a report. Structured evidence
records path, SHA-256, command (null if not recorded here), mode, target, result
and verification pin. `remaining_gaps` prevents scoped closure from hiding debt.
All recorded implementation work predates the R4 base, so
`preexisting_at_baseline` is true; this flag never fabricates an introducing SHA.

The old 24-row repair ledger stays byte-identical. Its snapshot hash and every
original field survive the migration under `historical_record`. Old PASS remains
a historical claim; missing introducing provenance stays null, with
`preexisting_at_baseline` and an explicit limit. Historical fix hashes not in the
current ancestry are not silently rewritten into new fixes or global PASS.
The original stdlib audit at `3dac3ac` is historical evidence, not current status.

Three local historical scratch references were checked and copied losslessly
into durable archives, with raw hashes in
[historical path resolution](evidence/historical-path-resolution.json).
The [accepted cleanup checkpoint](evidence/accepted-cleanup-checkpoint.json)
preserves the 2026-10-14 reported verification. The
[preservation manifest](evidence/preservation-manifest.json) protects preexisting
saved evidence; dated reports, logs and compatibility artifacts remain unchanged.

SHA reconciliation distinguishes `sha256` over exact Git blob bytes
(`hash_basis: git_blob_bytes`) from `acquired_workspace_sha256` over the bytes
read on Windows. Git's text CRLF/LF conversion is not evidence corruption.
Where they differ, `acquired_bytes_archive` preserves the original bytes in a
lossless GZ; the validator checks both hashes and accepts only checkout newline
differences in the working text. Raw qualification log hashes always refer to
the decompressed acquisition bytes. The legacy source additionally has a raw
snapshot archive. Stage ledger/evidence edits before running the provenance
validator so its repository-byte check sees the intended version.

## R4 closing checks

Run `python docs/tools/validate_r4_ledger.py --require-unchanged-implementation`, regenerate docs with
`python docs/tools/build_v01_docs.py`, then run
`python docs/tools/validate_v01_docs.py`. Save validation results and the exact
documentation verification commit separately from the closure commit. Require
no compiler/runtime/test tree changes, loss of old evidence, or release-ready
claim. On PASS, fast-forward the candidate to the R4 closure commit. Its exact
SHA is reported after committing, avoiding a self-referential hash in this file.
No main merge, release tag, or automatic R5 execution is authorized here.
