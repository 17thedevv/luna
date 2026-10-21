<!-- luna-doc-role: guidance -->

# Luna 0.1.0 stable execution

Started 2026-10-21 at the maintainer's request to execute the
[stable plan](../../plans/0.1.0-stable.md). **IN PROGRESS / NOT QUALIFIED**.

The integration baseline is `c13a9852279666a56526968d4b61622bf7310bb8`,
merging the plan and Lunar head `483b03d3` without changing main or the alpha tag.
It contains compiler25/metadata9/MVIR5/format2 and 49 manifest providers.
[Baseline provenance](baseline.json) records immutable inputs and source trees.
It is not a stable qualification result.

## Current work

| Milestone | State |
| --- | --- |
| S0 baseline/backlog | Integrated baseline recorded; publication-state contradictions repaired; grammar/compatibility reconciliation in progress |
| S1 correctness | DEBUG removal, quiet diagnostics and borrow maturity passed locally at `0387a15d`; reported I/O example/diagnostic repair under validation |
| S2 Lunar | Integrated; first candidate smoke failed with raw GCC on a Unicode path; retry uses the pinned LLVM/GNU wrapper and retains the failed evidence |
| S3 packaging/license/docs | GPL-3.0-only applied to first-party source; notices/package source inventory and the wider website example audit remain open |
| S4 exact-pin qualification | Not started; no prior-pin result transferred |
| S5 RC/use/release review | Not started; no stable tag or publication |

[Work backlog](backlog.json) gives each `STABLE-*` item an owner, source paths,
gate, state and evidence. [Scope matrix](scope-matrix.json) preserves all 25 R4
wave/deferral records and 24 legacy repair records without replacing their
historical identities or treating a historical PASS as a current execution.

The [first local preparation record](evidence/windows-preparation-0387/summary.json)
includes the successful runtime ABI, CLI/sysroot, quiet CLI, borrow and Lunar
unit checks, and the failed candidate smoke. It is **not** an eight-lane result.
The reported `println("hello world")` example conflicts with the existing
`&[u8]` API: literal `str` uses `println_str`. STABLE-015 tracks the EN/VI I/O
repair and source/artifact regression; remaining incorrect website examples
stay open under STABLE-009. The user's installed alpha compiler is unchanged.

The alpha [release closure](../../audits/r5-cpu-baseline-2026-10-20/release-closure.json)
supersedes the old `8525067f` CPU-package blocker for alpha only. Its eight
lanes and two clean package smokes are complete in that recorded scope.
Historical raw logs, archives, package SHAs and typed failure records remain
unchanged. Metadata-only Git history mappings are separate provenance; stable
must build and test a new immutable final pin.

## Release boundaries

Supported native release scope remains Windows x86_64 GNU and Ubuntu 24.04
x86_64 GNU. Lunar's Rust host identity is separate from the Luna target.
ASan claims remain limited to instrumented C runtime/tests. Provider counts,
workspace green totals and ancillary macOS checks do not certify every public
API/domain or expand the advertised target matrix.

Current accepted non-blocking deferrals are C4-FU1 dependency precision and
D1-FU1 presentation, under their original reopening conditions. Standalone
comptime statements remain unsupported execution with spanned E4005 rejection.
No language requirement is removed by migrating a record into this ledger.

Stable requires no open P0/P1 and no correctness/contract violation within the
advertised scope, plus complete final-pin qualification, package/installer
consumer tests, truthful version labels and a maintainer release review.
