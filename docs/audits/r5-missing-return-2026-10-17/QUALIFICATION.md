# Exact-pin qualification — 52ef3711

Implementation pin: `52ef3711813ac0e2ded272f5ebe71897752766bc`.
Harness commit: `0301ff9e824299f95b06a63e297fe08bae40a6a6`.
Protocol: compiler25 / metadata9 / MVIR5 / format2.

**IN PROGRESS — release readiness is not established.**
The candidate remains at `c085449b`; no result is carried from an earlier pin.

| Required evidence | Current result | Evidence |
| --- | --- | --- |
| Windows fresh detached clone and runtime ABI | PASS | `D:/r5-52ef3711/windows/provenance.json`, `runtime-abi.json` |
| Windows CLI debug/release and 49 providers | IN PROGRESS | Same lane; debug build passed, release build running |
| Windows focused and structural diagnostics | PENDING | Exact-pin runner continues after build |
| Windows docs, quickstart and comptime/layout | PENDING | Exact-pin runner continues after focused |
| Windows workspace triage and default-parallel final | PENDING | Both complete invocations required; no partial run is PASS |
| Ubuntu GNU normal and target contract | IN PROGRESS | [CI run 19](https://github.com/17thedevv/luna/actions/runs/39031984873) |
| Ubuntu comptime-vm | IN PROGRESS | Same CI run |
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

## Remaining closure work

Collect and verify the other exact-pin lane results, resolve failures if any,
then reconcile the report, public documentation, target matrix and limitations.
Only advance the candidate when every required gate is proven. Tagging, merging
main, creating a PR and publication remain maintainer decisions.
