# Exact-pin qualification — 52ef3711

Implementation pin: `52ef3711813ac0e2ded272f5ebe71897752766bc`.
Harness commit: `0301ff9e824299f95b06a63e297fe08bae40a6a6`.
Protocol: compiler25 / metadata9 / MVIR5 / format2.

**IN PROGRESS — release readiness is not established.**
The candidate remains at `c085449b`; no result is carried from an earlier pin.

| Required evidence | Current result | Evidence |
| --- | --- | --- |
| Windows fresh detached clone and runtime ABI | PASS | `D:/r5-52ef3711/windows/provenance.json`, `runtime-abi.json` |
| Windows CLI debug/release and 49 providers | PASS | `D:/r5-52ef3711/windows/build-summary.json`, `sysroot-provenance.json` |
| Windows focused and structural diagnostics | IN PROGRESS | Ownership batch PASS; remaining batches and structural audit follow |
| Windows native CI at the exact pin | IN PROGRESS | [Windows CI run 1](https://github.com/17thedevv/luna/actions/runs/39032926577) |
| Windows docs, quickstart and comptime/layout | PENDING | Exact-pin runner continues after focused |
| Strict A5 native bounds-trap replay | PENDING | `verify_bounds_trap.py`, after Cargo completes; require L002 and actual index/length in both profiles and provider modes |
| Windows workspace triage and default-parallel final | PENDING | Both complete invocations required; no partial run is PASS |
| Ubuntu GNU normal and target contract | IN PROGRESS | [CI run 19](https://github.com/17thedevv/luna/actions/runs/39031984873) |
| Ubuntu complete workspace triage and default-parallel final | IN PROGRESS | [Workspace run 1](https://github.com/17thedevv/luna/actions/runs/39032654737); normal's `--skip-workspace` does not satisfy this gate |
| Ubuntu comptime-vm | PASS IN EXECUTED SCOPE | [Verification](evidence/ubuntu-comptime-vm-verification.json), [original artifact](evidence/ubuntu-comptime-vm.zip) |
| Ubuntu runtime ASan | PASS IN EXECUTED SCOPE | [Verification](evidence/ubuntu-asan-verification.json), [original artifact](evidence/ubuntu-asan.zip) |

## Ubuntu ASan verification

The downloaded artifact's SHA-256 matches GitHub's published digest:
`b7245344671600105fea826a15f6a2dec1f7881bf9476717ca74dab3a93f9e32`.
All 42 files in its manifest and the compressed/raw hashes of all 19 gate logs
were checked. `git symbolic-ref -q HEAD` is the one expected nonzero exit (1),
proving a detached checkout; all other gate commands exit 0.

The archived provenance identifies the exact implementation and harness pins,
Ubuntu 24.04.5 x86_64, a clean initial checkout and the selected source trees.
The verbose RuntimeAbiTests log records partial-read cleanup and all invariant
tests passing, with no AddressSanitizer/LeakSanitizer report. The instrumentation
check and scope record confirm libasan linkage and
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1`.

This verifies the sanitized C runtime and RuntimeAbiTests only. Generated Luna
executables, compiler and stdlib are not claimed to be fully instrumented.

## Ubuntu comptime-vm verification

The original artifact's SHA-256 matches GitHub's published digest:
`86608bf37a32bdd940b19e25aa377841f407fbccaa1449605a9b7ff794ab1d40`.
All 981 manifest files and the compressed/raw hashes of all 28 gate logs were
checked at acquisition. The archive preserves the full downloaded evidence;
the extracted directory contains the summaries and key target/identity logs.

The nested comptime harness reports **88 PASS, 0 FAIL, 8 CHARACTERIZED** at the
same exact pin. The eight characterizations concern historical final-if
statement spelling; they grant no value-producing if-expression conformance
claim and are not counted as passes. The target gate verifies canonical
`x86_64-unknown-linux-gnu` artifacts independently of LLVM host spelling
`x86_64-pc-linux-gnu`, with 49 valid native provider bundles. The protected
compiler/runtime/test tree diff is empty.

This is the named 64-bit GNU comptime/layout boundary scope. It does not certify
other widths, non-GNU layout or standalone statement-position comptime execution.

## Ubuntu workspace acquisition

The original R5 checklist and target matrix require complete workspace evidence
on Ubuntu as well as Windows. The normal lane deliberately omits workspace to
fit a hosted job's time budget, so it cannot substitute for that requirement.
The dedicated workspace workflow uses two separately acquired fresh clones of
the same implementation pin. The final job waits for triage to pass, then runs
`cargo test --workspace` with default parallelism. Each job independently builds
the runtime, both CLI profiles and all 49 providers before running tests.

Harness commit: `185e977328d4c2839b7fbf4903af89ce4cd0a0ae`.
The recorder requires a nonempty completed test summary, zero failures, the
E3004 suite in the Cargo log and a clean final checkout; it preserves compressed
raw logs, command exits, hashes and toolchain/target identities on either result.
This is additional acquisition for `52ef3711`, not a new implementation pin.

## Remaining closure work

Collect and verify the other exact-pin lane results, resolve failures if any,
then reconcile the report, public documentation, target matrix and limitations.
Only advance the candidate when every required gate is proven. Tagging, merging
main, creating a PR and publication remain maintainer decisions.

## Windows CI acquisition

The checklist also requires Windows native CI identity and job URLs. The local
fresh-clone lane supplies its own native and full-workspace evidence, but does
not by itself prove that CI requirement. A dedicated Windows job now repeats
native build/runtime/sysroot, all focused batches, structural diagnostics,
documentation examples, quickstart and comptime acquisition at the same pin.
Complete Windows workspace triage/final remain required from the local lane.

CI harness commit: `23ff7e8aea176fe4ff44d0104fe2e84ffd083a2c`.
The job acquires the development distribution from the official
[LLVM 18.1.8 release](https://github.com/llvm/llvm-project/releases/tag/llvmorg-18.1.8),
records the acquired archive hash and verifies required tools/import library.
It verifies the actual GNU compiler target and rebuilds the native runtime;
the [Windows runner image](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)
is only toolchain setup context, not conformance evidence. The Rust compiler's
MSVC host identity does not advertise a Luna MSVC target.
