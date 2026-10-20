# R4 — Release target qualification matrix

The [adopted matrix](../../../spec/0.1/conformance.md) still release-gates
Windows GNU and Ubuntu GNU. Contract scope and certification are separate.
**Release: BLOCKED / NOT VERIFIED.**

| Target / profile | Existing qualification | Pin and evidence | R5 still required |
| --- | --- | --- | --- |
| x86_64-pc-windows-gnu, native; debug/release compiler CLI | VERIFIED IN TESTED SCOPE | e4f9fb530d72628c81dfa29d117afeb99cc7664c; 236/236 arithmetic acquisitions; three workspace runs, each 1404 passed / 0 failed / 1 ignored / exit0 | Clean exact R5 candidate, native runtime ABI, focused/parity/quick-start and workspace evidence, Windows CI |
| x86_64-unknown-linux-gnu, Ubuntu normal | NOT VERIFIED | No exact-candidate native qualification accepted here | Build compiler/runtime/sysroot, ABI, Whole-File I/O, formatting/Unicode, public CLI/parity and workspace; retain CI job URLs, pin, exits and hashes |
| x86_64-unknown-linux-gnu, Ubuntu ASan | NOT VERIFIED | No accepted exact-candidate ASan run | Execute and record sanitized runtime ABI/partial-read cleanup; state the instrumented components and test scope; do not promote a narrow runtime check to all language/stdlib sanitizer coverage |
| macOS, freestanding, other architectures/targets | NOT ADVERTISED | Historical/runtime-only probes do not advertise support | Separate maintainer adoption and qualification if added; not an automatic R5 expansion |

Windows evidence: [native object/header manifest](../evidence/e1-final5-current-native-target-2026-10-13.json),
[49 fresh bundles](../evidence/e1-final5-sysroot-provenance-2026-10-13.json),
[workspace summary](../evidence/e1-workspace-final6-2026-10-13.json), and
[E-wave closure](../evidence/REPORT-EWAVE-CLOSURE-2026-10-13.md).
These establish actual COFF objects and GNU LLIB target headers, separately
from the Rust host triple. Windows MSVC as a Rust host does not certify a Luna
MSVC target. CLI Cargo profiles also do not imply a separate native optimization
flag: E1 native emission uses the current LLVM default optimization level.

## Existing CI is configuration, not accepted evidence

[Phase 5 workflow](../../../../.github/workflows/phase5-platform.yml) defines
Ubuntu runtime ABI, Whole-File I/O, a runtime ASan job and workspace work, with
additional macOS runtime jobs. [Phase 6](../../../../.github/workflows/phase6-formatting.yml)
defines formatting/Unicode and Ubuntu workspace checks. Their existence does
not prove execution at the R4/R5 candidate. A docs-only push may not match their
path filters. R5 must deliberately dispatch/obtain results for the exact pin;
older branch runs, a job definition, skipped jobs and a metadata-only target
probe cannot be substituted for native qualification.

The current ASan job instruments the C runtime and runs RuntimeAbiTests. It
does not instrument the Rust compiler or automatically establish sanitizer
coverage for every generated Luna executable. Broader sanitizer scope needs
an explicit executable plan and its own evidence; retain that boundary in the
final R5 report. There is no automatic Windows release gate in those Ubuntu/
macOS workflow definitions; obtaining exact Windows native CI remains R5 work.
