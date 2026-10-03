<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](docs/spec/0.1/README.md).

# Luna 0.1 status — 2026-10-03

Contract baseline: [spec/0.1](docs/spec/0.1/README.md). Release conformance:
**BLOCKED / NOT VERIFIED**. Compiler revision evaluated: `3dac3ac`.

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
