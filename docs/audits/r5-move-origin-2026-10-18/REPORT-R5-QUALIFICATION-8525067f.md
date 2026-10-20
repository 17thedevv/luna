# R5 qualification closure — integrated pin 8525067f

Date: 2026-10-20. **READY FOR RELEASE REVIEW — qualification COMPLETE in the tested adopted scope.**
Full release publication and clean release-package smoke remain pending.

Tested implementation and remote candidate: `8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5`.
Candidate fast-forwarded from `c085449b` without force; remote checked 2026-10-20.
Main remains `401a95a872f0fdfb9840640f41cc4ac629cb17cd`.
Versions: **compiler 25 / semantic metadata 9 / MVIR 5 / format 2**.

Introducing fixes: `67146195` (move origins), `d3d0ffe8` (identity/order),
`ce886d09` (declaration/implementation anchors), integrated by `8525067f`.
Earlier DIAG/comptime/return/Linux-link repairs remain included.
Harnesses: Ubuntu `aa3ed8abc2f9001e35530ca3ed473191c40a80ac`;
Windows `896876d8694a4010d4f050187afe0af774b2b4a7`.
Report/evidence descendants are separate from the tested implementation.

## Eight exact-pin lanes

| Lane | Authoritative CI job | Verified result |
| --- | --- | --- |
| Windows native normal | [118141683149](https://github.com/17thedevv/luna/actions/runs/39276394971/job/118141683149) | Fresh clean clone; runtime ABI; debug/release CLI; 49 GNU providers; 12 focused batches; diagnostics; docs/examples; native quickstart; comptime 88 PASS/0 FAIL/8 characterized; strict L002 4/4 |
| Windows workspace triage | [118141682645](https://github.com/17thedevv/luna/actions/runs/39276394971/job/118141682645) | Exit0; 236 groups; 1421 passed/0 failed/1 ignored |
| Windows workspace final | [118243423793](https://github.com/17thedevv/luna/actions/runs/39276394971/job/118243423793) | After triage, default parallelism; exit 0; 236 groups; 1421/0/1 |
| Ubuntu native normal | [117679870941](https://github.com/17thedevv/luna/actions/runs/39128925234/job/117679870941) | 48 gate records PASS; runtime/CLI/49 providers; focused/parity/diagnostics; docs/examples and native quickstart |
| Ubuntu runtime ASan | [117679870857](https://github.com/17thedevv/luna/actions/runs/39128925234/job/117679870857) | 19 records PASS; libasan.so.8; partial-read cleanup executes; RuntimeAbiTests 100%; no sanitizer/leak report |
| Ubuntu comptime/VM | [117679870976](https://github.com/17thedevv/luna/actions/runs/39128925234/job/117679870976) | 28 records PASS; nested 88 PASS/0 FAIL/8 characterized |
| Ubuntu workspace triage | [117679870999](https://github.com/17thedevv/luna/actions/runs/39128925234/job/117679870999) | Exit0; 236 groups; 1421/0/1 |
| Ubuntu workspace final | [117720597415](https://github.com/17thedevv/luna/actions/runs/39128925234/job/117720597415) | After triage, default parallelism; exit 0; 236 groups; 1421/0/1 |

All four workspace runs ignore only `mutable_references::sem_mutref_06_invalid_fixture`.
These are the pin's actual counts; no historical PASS is transferred.

## Evidence acceptance

All eight original ZIP byte streams are retained in
[archives](evidence/pin-8525067f/archives/), with
[original GitHub metadata](evidence/pin-8525067f/ci-artifact-manifest.json).
Every ZIP SHA-256, size and CRC matches. Signed URLs are excluded.

The [independent verifier](../../tools/verify_ci_artifacts.py) checks pins,
clean initial/final state, detached acquisition, source trees, gate exits and
log digests. Ubuntu: 1270 file hashes plus 153 compressed and153 raw log hashes.
Windows: 413 distinct recorded log/archive/stream hashes. Windows normal also
checks 98 archived provider bundle copies against library/object hashes, LLIB
headers and COFF 0x8664. Canonical Linux provider records are ELF64 x86_64 with
target `x86_64-unknown-linux-gnu`; LLVM's host spelling is provenance.

Ubuntu workspace records code-tree identity twice with identical command,
exit and content hashes; only elapsed seconds differ. The verifier records
this acquisition detail. Six rejection controls covered ZIP tampering, wrong
pins, inventory tampering, raw-log mismatch and falsified workspace totals.

Hosted bounds replay exits with Windows status 3221226505 in all four cases,
each accompanied by **luna: PANIC [L002], index 3, length 2**. The earlier local
exit 3 replay is separate evidence; an arbitrary crash cannot pass.

The [final review](evidence/pin-8525067f/closure-review.json) maps DIAG-1..10,
source hashes and regressions. Declaration-location coverage passes all four
workspaces and the local focused matrix. Hosted focused diagnostics includes
move-origin, per-code and missing-return suites. Both native example gates
have 14 satisfied contract rows and two legacy characterizations.
Local integrity verification checks 38 log/archive digests, zero mismatch or
missing, excluding five interrupted records; failed tooling attempts remain visible.

Reproduce from the repository root:

```text
python docs/tools/verify_ci_artifacts.py --manifest docs/audits/r5-move-origin-2026-10-18/evidence/pin-8525067f/ci-artifact-manifest.json --pin 8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5 --out OUTPUT.json
```

## Scope and remaining full-goal work

[Requirements](REQUIREMENTS.md) · [Targets](TARGET-MATRIX.md) ·
[Limits](LIMITATIONS.md) · [Lane ledger](evidence/pin-8525067f/lane-state.json) ·
[Full archive verification](evidence/pin-8525067f/all-artifact-verification.json).

Inventory: 72 codes, 64 produced, eight reserved without producers, four produced
conditions without portable behavioral fixture triggers, undeclared coverage debt 0.
**Retraction:** E3004 is produced and tested at this pin; the earlier
no-producer claim came from a divergent base.

Superseded pins and local interrupted/PDB attempts remain historical.
No new semantic freeze, target expansion or blanket API certification is declared.
Qualification is complete. Remaining: maintainer PR/main/tag/publication decision,
packages from the chosen verified release commit, clean Windows/Ubuntu package
smoke, final tag/assets/checksum/release-note reconciliation.
See the concrete [handoff](RELEASE-HANDOFF.md).
