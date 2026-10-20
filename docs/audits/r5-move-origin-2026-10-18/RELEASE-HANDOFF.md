# Luna 0.1-alpha.1 — maintainer review handoff

**CURRENT REPLACEMENT STATUS — 2026-10-20:** `b5cb77c3` has 8/8 qualification
lanes and all eight ZIPs independently verified. Windows workspace final PASS
(236 groups, 1422/0/1). Both clean package smokes PASS from `cd9cd188`, and the
current PR merge-preview full workspace passes, including the closure control
that failed once in an earlier run. Candidate remains `8525067f`, main
`401a95a8`, and PR #7 is ready for the authorized merge and release sequence. See the
[CPU baseline repair and current evidence](../r5-cpu-baseline-2026-10-20/README.md)
and its [merge-preview replay record](../r5-cpu-baseline-2026-10-20/evidence/pin-b5cb77c3/pr-merge-preview-replay.json).

**SUPERSEDED FOR RELEASE — 2026-10-20.** Eight-lane qualification at
`8525067f` remains valid in its executed scope, but its clean Windows package
smoke failed on a different CPU. The replacement pin now closes all eight lanes
and archive checks. The full goal remains active until merge and publication.

Candidate/tested implementation: `8525067f5e6b7c8d711e9df0cb5acf9606b1b0e5`.
Candidate fast-forwarded from `c085449b` without force.
Main remains `401a95a872f0fdfb9840640f41cc4ac629cb17cd`.
Evidence/docs branch: `r5-diag-repair`; its implementation trees match the pin.

[Report](REPORT-R5-QUALIFICATION-8525067f.md) ·
[Eight verified archives](evidence/pin-8525067f/all-artifact-verification.json) ·
[Requirements](REQUIREMENTS.md) · [Targets](TARGET-MATRIX.md) · [Limits](LIMITATIONS.md).

Eight hosted lanes pass. Four workspaces each 1421 passed/0 failed/1 ignored;
the ignored test is `mutable_references::sem_mutref_06_invalid_fixture`.
Strict L002 4/4. Comptime per host 88 PASS/0 FAIL/8 characterized.
Both normal lanes rebuild49 providers. ASan covers the C runtime only.
Versions:compiler 25/metadata 9/MVIR 5/format 2.

## Proposed PR

Title: **Qualify Luna 0.1-alpha.1 on Windows GNU and Ubuntu GNU**.
Base: `main`. Head: existing `r5-diag-repair` after the evidence commit.

PR description:

This PR advances the Luna 0.1 candidate with the qualified portable CPU baseline
and its release documentation. All eight exact-pin Windows/Ubuntu qualification
lanes and their CI ZIPs are independently verified. The Windows and Ubuntu
packages also pass clean hosted consumer smoke tests from the exact packaged
source commit `cd9cd1884d2e49c0f03979597ca73e92043b7fe7`.

Validation: four complete workspaces report 236 groups, 1422 passed, 0 failed,
1 ignored; both normal lanes verify 49 providers per target; the Windows native
lane passes strict L002 4/4; comptime/VM reports 88 pass and 8 characterized;
Ubuntu runtime ASan passes. Exact pins, jobs, archive digests and limits are in
the report and machine-readable manifest.

## Proposed release and package checks

Authorized tag: `v0.1.0-alpha.1` (human approval received 2026-10-20).
Two packages use the canonical target names. Each contains compiler CLI,
native runtime, 49 canonical LLIB/object bundles, protocol/target/source/build
manifest, checksums, license notice and usage instructions. Document LLVM18 and GNU
toolchain prerequisites explicitly.

Build from the selected verified release commit. Smoke each downloaded package
in a separate clean Windows/Ubuntu job without a source checkout or external
sysroot: verify identities/hashes, then build/run the public quickstart and
a stdlib provider case using the package runtime/sysroot. Preserve all evidence.

Before tagging, prove release code-tree identity against the qualified pin;
requalify if implementation or artifact identity changes. Reconcile tag,
assets/checksums and release notes, then write the final closing record.

## Remaining authority and work

Human authorization received 2026-10-20: create PR, merge and publish
`v0.1.0-alpha.1` after Windows/Ubuntu package smoke PASS. [PR #7](https://github.com/17thedevv/luna/pull/7)
is open as a draft. No merge/tag/publication has happened. The new package workflow
builds an immutable source commit with the same qualified implementation trees;
separate jobs verify the downloaded packages without a source checkout.
The tag will identify the exact packaged source commit, with main integration
recorded separately. [Authorization record](evidence/pin-8525067f/release-authorization.json).

No project LICENSE file exists in the source snapshot. Packaging preserves that
state with a license notice rather than inventing a license grant. CLI output
remains `luna 0.1.0`; package manifests identify distribution `0.1.0-alpha.1`.
