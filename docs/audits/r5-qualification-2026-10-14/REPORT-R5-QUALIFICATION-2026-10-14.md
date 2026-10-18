# R5 — exact release qualification (9bda6f57)

**Tested commit:** `9bda6f572fa5f9904379c08c625c28122fcd0c56` (candidate `luna-0.1-alpha.1-candidate`)
**Protocol:** compiler25 / metadata9 / MVIR5 / format2
**Verdict:** **BLOCKED**

Qualification ran on a fresh detached clone of the exact candidate with all build
and log output outside the checkout. No lane modified the candidate; the
diagnostics lane added no tests and the replay window reused the freshly built
exact-pin CLI.

## Reconciliation

| Gate | Windows | Ubuntu | Required |
| --- | --- | --- | --- |
| Fresh pin | PASS | PASS | PASS |
| Runtime ABI | PASS | PASS | PASS |
| 49 providers | PASS | PASS | PASS |
| Focused A/C/D/E | PASS | NOT RUN (blocked by target gate) | PASS |
| DIAG-1..10 | BROKEN | shared | PASS |
| Docs examples 16/16 | PASS | NOT RUN (blocked by target gate) | PASS |
| Docs validator | PASS | NOT RUN (blocked by target gate) | PASS |
| README quick start | PASS | NOT RUN (blocked by target gate) | PASS |
| Workspace | PASS | NOT RUN (blocked by target gate) | PASS |
| ASan runtime ABI | N/A | PASS | PASS |

## Windows GNU lane

- Runtime MinGW/Ninja Release + `RuntimeAbiTests`: exit 0.
- CLI debug/release built from the exact clone; sysroot rebuilt: 49 provider bundles,
  protocol 25/9/5/2, target `x86_64-pc-windows-gnu`,
  COFF 0x8664.
- Focused A/C/D/E batches: PASS (11 batches, all exit 0).
- Docs conformance recorder: exit 0; docs validator exit 0.
- README quick start build+native: PASS (PE64/COFF64 0x8664).
- Workspace triage: exit 0 (231 groups,
  1404 passed / 0 failed).
- Workspace final: exit 0 (231 groups,
  1404 passed / 0 failed).

## Ubuntu GNU lane

- Normal: **BLOCKED by a target contract conflict.** Actual `.llib`/manifest target
  `x86_64-pc-linux-gnu` versus advertised
  `x86_64-unknown-linux-gnu` (docs/spec/0.1/conformance.md:66 advertised unknown-linux-gnu; other targets unadvertised). 49 providers,
  protocol 25/9/5/2, ELF64 objects, 54 file hashes and
  24 gate-log hashes verified with 0 mismatches.
  Dependent focused/docs/workspace gates were intentionally not run. Resolving this
  requires either a documentation reconciliation of the advertised triple or a
  compiler target change (new implementation pin); it is a maintainer decision.
- ASan: **PASS in C runtime ABI / partial-read cleanup scope only**
  (42 file hashes, 0 mismatches);
  no broader sanitized compiler/stdlib claim.

## R5-2 structural DIAG-1..10 audit

**BROKEN — the structural diagnostic gate is not satisfied.**

| Invariant | Verdict |
| --- | --- |
| DIAG-1 | BROKEN |
| DIAG-2 | PARTIAL |
| DIAG-3 | BROKEN |
| DIAG-4 | BROKEN |
| DIAG-5 | PARTIAL |
| DIAG-6 | UNVERIFIED |
| DIAG-7 | PARTIAL |
| DIAG-8 | BROKEN |
| DIAG-9 | BROKEN |
| DIAG-10 | PARTIAL |

Confirmed structural findings (source traces, with existing reducers and the
exact-pin replay): untyped user-visible I/O error path (DIAG-1); move/loan
producers omit antecedent related labels (DIAG-3/DIAG-9); analyzer dedup and some
identity assertions depend on message text (DIAG-4); TypeChecker constructs a
Resolver code E1008, Borrowck constructs TypeChecker E2016, Mono constructs
TypeChecker E2001 (DIAG-8). Reported findings: 7.
The replay matched 40/40 expectations, which
confirms reproducers rather than conformance. Repair requires compiler/runtime/
test changes and therefore a new implementation pin; the candidate is unchanged.

## Evidence

Packed copies with SHA-256 in `evidence/evidence-sha256.json`. Raw lane outputs
and gzip workspace logs remain outside the checkout as recorded by the lanes.

## Release decision

**R5: BLOCKED.** Release remains **BLOCKED** pending a maintainer decision on
(1) the structural diagnostic gate and (2) the Ubuntu target contract. No tag was
created, `main` was not touched, and the candidate was not modified by R5.
