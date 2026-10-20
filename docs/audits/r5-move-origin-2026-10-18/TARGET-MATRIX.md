# Luna 0.1-alpha.1 qualification target matrix

**Historical pin only; current release qualification is pending at `b5cb77c3`.**
The clean Windows package failure superseded this release verdict; see the
[replacement qualification](../r5-cpu-baseline-2026-10-20/README.md).

Date: 2026-10-20. Implementation/candidate: `8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5`.
**READY FOR RELEASE REVIEW**, publication pending.

| Canonical target | Evidence | Accepted scope |
| --- | --- | --- |
| `x86_64-pc-windows-gnu` | Run 39276394971; three verified ZIPs; 49 LLIB/COFF 0x8664 providers; runtime/CLI/parity; workspace triage/final each 1421/0/1 | Windows x86_64 GNU native. Rust MSVC host does not replace emitted GNU identity. |
| `x86_64-unknown-linux-gnu` | Run 39128925234; five verified ZIPs; 49 canonical ELF64 x86_64 provider records; native focused/parity/VM/runtime; workspace each 1421/0/1 | Ubuntu x86_64 GNU. LLVM host spelling `x86_64-pc-linux-gnu` is provenance. |
| Ubuntu runtime ASan | libasan.so.8; RuntimeAbiTests with partial-read cleanup;19 gates PASS | Sanitized C runtime only. |

Versions:compiler 25/metadata 9/MVIR 5/format 2. VM/layout evidence is bounded to
the named 64-bit GNU cases. macOS, other architectures, MSVC Luna targets and
freestanding targets remain unadvertised. Ancillary passes do not expand this matrix.

[Archive verification](evidence/pin-8525067f/all-artifact-verification.json) ·
[Report](REPORT-R5-QUALIFICATION-8525067f.md) · [Limits](LIMITATIONS.md).
