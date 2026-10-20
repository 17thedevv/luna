# Luna 0.1-alpha.1 — maintainer review handoff

**CURRENT RELEASE STATUS — 2026-10-20:** Luna `v0.1.0-alpha.1` is published as
a prerelease. `b5cb77c3` has 8/8 qualification lanes and all eight ZIPs
independently verified. Windows workspace final PASS (236 groups, 1422/0/1).
Both clean package smokes PASS from `cd9cd188`; PR #7 merged as `97422d2c` after
the PR merge-preview full workspace and formatting checks passed. Candidate is
fast-forwarded to `b5cb77c3`; the annotated tag points to exact package source
`cd9cd188`, which is an ancestor of main. See the
[CPU baseline repair and current evidence](../r5-cpu-baseline-2026-10-20/README.md)
and its [release closure record](../r5-cpu-baseline-2026-10-20/release-closure.json).

**SUPERSEDED FOR RELEASE — 2026-10-20.** Eight-lane qualification at
`8525067f` remains valid in its executed scope, but its clean Windows package
smoke failed on a different CPU. The replacement pin closes all eight lanes and
archive checks; the authorized merge and publication are complete.

Current tested implementation: `b5cb77c3286f583c43c41d0e56f4136903ebb842`.
Candidate `luna-0.1-alpha.1-candidate` was fast-forwarded from `8525067f`.
Main merge commit: `97422d2cb5feaac0361c14e620843785dd09615b`.
Packaged source and annotated tag target: `cd9cd1884d2e49c0f03979597ca73e92043b7fe7`.

[Report](REPORT-R5-QUALIFICATION-8525067f.md) ·
[Eight verified archives](evidence/pin-8525067f/all-artifact-verification.json) ·
[Requirements](REQUIREMENTS.md) · [Targets](TARGET-MATRIX.md) · [Limits](LIMITATIONS.md).

Eight hosted lanes pass. Four workspaces each 1421 passed/0 failed/1 ignored;
the ignored test is `mutable_references::sem_mutref_06_invalid_fixture`.
Strict L002 4/4. Comptime per host 88 PASS/0 FAIL/8 characterized.
Both normal lanes rebuild49 providers. ASan covers the C runtime only.
Versions:compiler 25/metadata 9/MVIR 5/format 2.

## Merged PR

Title: **Qualify Luna 0.1-alpha.1 on Windows GNU and Ubuntu GNU**.
PR #7 merged into `main` on 2026-10-20 at `97422d2cb5feaac0361c14e620843785dd09615b`.

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

## Published release and package checks

Authorized tag: `v0.1.0-alpha.1` (human approval received 2026-10-20).
The [prerelease is published](https://github.com/17thedevv/luna/releases/tag/v0.1.0-alpha.1).
The tag points to the exact package source commit. Two packages use the canonical target names. Each contains compiler CLI,
native runtime, 49 canonical LLIB/object bundles, protocol/target/source/build
manifest, checksums, license notice and usage instructions. Document LLVM18 and GNU
toolchain prerequisites explicitly.

Both packages passed clean hosted Windows/Ubuntu consumer smokes. The published
ZIPs and `SHA256SUMS` were downloaded and their API digests, sizes and package
checksums verified. The tag target's compiler/runtime/test trees match the
qualified implementation pin.

## Authorization and completed actions

Human authorization received 2026-10-20: create PR, merge and publish
`v0.1.0-alpha.1` after Windows/Ubuntu package smoke PASS. [PR #7](https://github.com/17thedevv/luna/pull/7)
is merged; the candidate advanced, the exact source tag was pushed, and the
prerelease assets were published and verified. [Authorization record](evidence/pin-8525067f/release-authorization.json)
and [release closure](../r5-cpu-baseline-2026-10-20/release-closure.json) preserve
the evidence. No qualification, merge or publication work remains.

No project LICENSE file exists in the source snapshot. Packaging preserves that
state with a license notice rather than inventing a license grant. CLI output
remains `luna 0.1.0`; package manifests identify distribution `0.1.0-alpha.1`.
