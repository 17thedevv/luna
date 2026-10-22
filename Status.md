<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** The [versioned specification](docs/spec/0.1/README.md) and adopted contracts govern; implementation evidence does not create semantic authority.

# Luna 0.1 status — 2026-10-23

**Current: alpha published; stable execution IN PROGRESS / NOT QUALIFIED.**
The [stable ledger](docs/stable/0.1.0/README.md) records the integrated Luna/Lunar
baseline, 49 historical scope records and outstanding work. The alpha tag now
points to `a3080712` after metadata-only history cleanup; original package/log
SHAs below are retained as historical build identities, not silently rewritten.

The 2026-10-22 [stdlib API expansion](docs/audits/stdlib-usize-2026-10-22/README.md)
passes focused Windows validation at `28802f12` on stdlib-redesign: usize memory counts, additional type
constructors, ordered-set algebra and fixed-width bit utilities. Compiler
protocol 26 / runtime ABI 2 / 50 providers form a new bundle requiring fresh
qualification. The [C++23 comparison](docs/proposals/stdlib-redesign-0.1/cpp-library-gap-map.md)
tracks missing functional families; graph and basic CPU matrix/tensor remain
planned. Eight CLI suites (29 tests), 40 existing ptr/mem/collection tests and
all 50 canonical artifact/object checks PASS. Ubuntu
[run 39724850945](https://github.com/17thedevv/luna/actions/runs/39724850945)
now reports SUCCESS, including public CLI, fresh sysroot, C-runtime ASan and
complete workspace/doctests at 28802f12. Artifact archive digest acquisition,
full Windows and final package/installer gates remain open.
These changes are not installed or included in the published alpha.

The [I/O redesign](docs/proposals/stdlib-redesign-0.1/io-api.md) now precedes
further expansion. The maintainer selected `$name` interpolation; API/error/
buffer policies and generic macro machinery are documented proposals, not
implemented support. Existing concatenation macros retain their current behavior.

## Published alpha snapshot — original build identities, 2026-10-20

**Luna 0.1.0-alpha.1 is published as a prerelease.**
The eight lanes at `8525067f` remain historical qualification evidence, but that
pin is not sufficient for distribution across machines. The portable CPU repair
at `b5cb77c3` has **8/8 lanes and all eight ZIP artifacts independently verified**.
Windows workspace final passed (236 groups, 1422/0/1). Both clean Windows/Ubuntu
package smokes at `cd9cd188` PASS. The annotated tag points to that exact
packaged source commit; main contains it as an ancestor after PR #7 merged.
[Finding, repair and current evidence](docs/audits/r5-cpu-baseline-2026-10-20/README.md).

Candidate/tested pin: `b5cb77c3286f583c43c41d0e56f4136903ebb842`, fast-forwarded
from `8525067f` without force. Main merge commit: `97422d2cb5feaac0361c14e620843785dd09615b`.
Release: [`v0.1.0-alpha.1`](https://github.com/17thedevv/luna/releases/tag/v0.1.0-alpha.1);
packaged source/tag target: `cd9cd1884d2e49c0f03979597ca73e92043b7fe7`.
Protocol: compiler / semantic metadata / MVIR / format **25 / 9 / 5 / 2**.

At the superseded implementation, all eight Windows/Ubuntu lanes PASS. Original ZIPs
are retained and independently verified. Four full workspace runs each have
**1421 passed / 0 failed / 1 ignored**. Both normal lanes build 49 canonical
providers. Windows strict L002 replay passes 4/4; comptime per host reports
88 PASS / 0 FAIL / 8 characterizations. Ubuntu ASan certifies the C runtime only.

[Final report](docs/audits/r5-move-origin-2026-10-18/REPORT-R5-QUALIFICATION-8525067f.md) ·
[Requirement matrix](docs/audits/r5-move-origin-2026-10-18/REQUIREMENTS.md) ·
[Ledger](docs/audits/r5-move-origin-2026-10-18/evidence/pin-8525067f/lane-state.json) ·
[Target matrix](docs/audits/r5-move-origin-2026-10-18/TARGET-MATRIX.md) ·
[Limits](docs/audits/r5-move-origin-2026-10-18/LIMITATIONS.md).

Inventory: 72 codes, 64 produced, eight reserved without producers, four declared
conditions without portable fixture triggers, undeclared coverage debt zero.
E3004 is produced and covered; the contrary finding was retracted. The earlier
`2/25/5/2` tuple wording was incorrect; metadata is 9.
Local interrupted/PDB attempts are historical, never passing evidence.

Maintainer authorized PR/main/tag/publication after package smoke PASS on 2026-10-20.
PR #7 is merged; the PR merge-preview full workspace and formatting checks PASS;
an earlier closure ICE did not recur and is preserved with its successful replay.
Qualification, candidate advancement, PR merge, tag and publication are complete.
The [release closure](docs/audits/r5-cpu-baseline-2026-10-20/release-closure.json)
records all SHAs and verified release-asset digests. This alpha publication does
not create a new semantic freeze.

## Prior reconciliation - 2026-10-14

The following snapshot describes its dated evidence, not the selected pin.

**Release: BLOCKED / NOT VERIFIED; R5 PENDING.** R4 documentation base is
`3f8646c654b99b67bae57ee290b7a7e23d717b40`; tested implementation is
`e4f9fb530d72628c81dfa29d117afeb99cc7664c`. Protocol: **25 / 9 / 5 / 2**
(compiler / semantic metadata / MVIR / artifact format).

| Area | Current scoped status / evidence |
| --- | --- |
| A-wave | CLOSED; closure/async/partial cleanup, cast, view and logical-not scopes |
| C-wave | CLOSED IN TESTED SCOPE; calls, impl metadata, projections, dependencies and freshness |
| D-wave | CLOSED with follow-ups classified; complete diagnostic structural audit remains R5 |
| E2 grammar | CLOSED; adopted grammar behavior reconciled, legacy syntax only characterized |
| E1 checked arithmetic | CLOSED / QUALIFIED IN TESTED SCOPE; 59 cases x four modes = 236/236 |
| Full workspace | Three final6 runs at the tested pin: each 1404 passed / 0 failed / 1 ignored / exit0 |
| Windows GNU | VERIFIED IN TESTED SCOPE; 49 fresh bundles with actual GNU LLIB/COFF object identity |
| Ubuntu GNU normal / ASan | NOT VERIFIED; required R5 release gates |
| Cleanup | Accepted CLOSED 2026-10-14; 254 temporary files removed; nine logs and referenced sysroot retained |
| R4 / R5 | R4 CLOSED in docs/evidence scope; R5 exact candidate qualification pending; no release-ready claim |

The [exact release ledger](docs/audits/0.1-alpha-completion-2026-10-04/r4/README.md)
maps every wave/follow-up to closure SHA and saved evidence. Its
[diagnostic inventory](docs/audits/0.1-alpha-completion-2026-10-04/r4/diagnostic-inventory.md),
[target matrix](docs/audits/0.1-alpha-completion-2026-10-04/r4/target-matrix.md),
[limitations](docs/audits/0.1-alpha-completion-2026-10-04/r4/known-limitations.md)
and [R5 checklist](docs/audits/0.1-alpha-completion-2026-10-04/r4/r5-checklist.md)
define the remaining qualification. Standalone comptime statement execution
is unsupported; VM layout is only verified in the 64-bit GNU tested scope.
C4-FU1 precision and D1-FU1 presentation remain approved non-blocking deferrals;
they do not waive correctness or DIAG-1..10 requirements.

## Historical audit — 2026-10-03

Contract baseline: [spec/0.1](docs/spec/0.1/README.md). Release conformance:
**BLOCKED / NOT VERIFIED**. Compiler revision evaluated: `3dac3ac`.

The table below describes that audit revision. All-worktree integration on
2026-10-03 adds later compiler/runtime/stdlib changes and a 49-provider manifest;
it does not transfer the audit's verdicts or old freeze coverage automatically.
See the [integration record](docs/integration/2026-10-03-all-worktrees.md).

| Area | Current evidence |
|---|---|
| Compiler CLI / sysroot | Builds; 32 providers rebuilt through the official path |
| Runtime | Windows x86_64 GNU ABI CTest passes; other profiles not certified |
| Existing stdlib suites | 33/34 complete; 258 tests in completed suites; storage suite times out |
| Existing CLI stdlib fixtures | 109/116 expected outcomes; actual dispatch failures and stale fixture expectations distinguished |
| Additional audit probes | 50 source/artifact attempts; deliberate failures characterize confirmed bugs |
| Correctness | 4 compiler defects, 7 stdlib defects and SliceIter coupling recorded in the audit |
| Optimization | HashMap collision scaling measured across 126 timed samples; broader performance remains partial |
| Language contracts | Retained even where the current implementation/evidence is partial |

See the [audit](docs/audits/stdlib-2026-10-02/README.md) for exact logs, limits and
severity, and the [0.1 gap register](docs/spec/0.1/gaps.md) for grammar/spec debt.
No full compiler, all-target, sanitizer or fuzzing completion is claimed.
Historical progress reports are indexed as historical evidence.

## Module implementation update — 2026-10-04

`using namespace_name;` and optional `luna.toml` provider bindings are implemented
on branch `codex/antigravity-repair-0.1`. The 57-fixture public CLI matrix passes
in source-only and freshly built artifact-only modes on Windows x86_64 GNU.
Aliases, existing imports, provider identity and strict artifact rejection are
preserved. See the [feature verification record](docs/audits/alpha-modules-2026-10-03/README.md)
for current source fingerprints, commands and regression failures. This update
does not replace the dated audit above or establish release conformance.

Provider configuration was tightened on 2026-10-04 to relative-only file values.
Absolute/drive/root/UNC/device paths and expansion markers reject with E6008;
internal resolved paths and CLI search paths may remain absolute. See the
[amendment verification](docs/audits/provider-config-relative-2026-10-04/README.md).
Release conformance remains blocked; this does not close the older workspace gaps.
