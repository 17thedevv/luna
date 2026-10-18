<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](docs/spec/0.1/README.md).

# Luna 0.1 status — 2026-10-18

**Release: BLOCKED / NOT VERIFIED; diagnostic origin/identity/order repairs under validation.**
Candidate: `c085449b207b5860b1d7a79bee221c2ff3d357c3`.
Protocol: compiler / metadata / MVIR / format **25 / 9 / 5 / 2**.

The [current report](docs/audits/r5-move-origin-2026-10-18/README.md) and
[replacement matrix](docs/audits/r5-move-origin-2026-10-18/REQUIREMENTS.md)
record the findings and remaining work. Previous pin `3a72f946` completed normal,
ASan and comptime lanes plus Windows/Ubuntu full workspace triage/final (each
1414 passed, 0 failed, 1 ignored). Its final audit found E3001 origins lost at
nested joins or retained after field reinitialization. Correct rejection alone
does not satisfy DIAG-3/9. Follow-up review found distinct E3002 events collapsed
by message-based dedup and unstable multi-function diagnostic order (DIAG-4/6).
The generic repairs are being validated before selecting a replacement pin and
reacquiring every lane.

Earlier diagnostic E4002/E4003, E3004 and Linux native math linkage repairs stay
in the implementation. Their previous passes remain scoped to their own SHA.
Tagging, main merge, PR creation and publication remain maintainer decisions;
no release-ready or freeze claim is made.

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
