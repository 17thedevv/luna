# Luna 0.1-alpha.1 — maintainer review handoff

**READY FOR RELEASE REVIEW —2026-10-20.** Qualification is complete in the
adopted tested scope. The full release goal remains active.

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

Proposed description:

The 0.1 candidate now includes typed comptime errors, originating diagnostics
under run, missing-return rejection, Linux native math linkage, deterministic
move/loan origins and diagnostic identity/order, plus structural declaration,
impl and candidate locations. Eight exact-pin Windows/Ubuntu lanes and original
ZIP/manifest/raw/compressed-log evidence are independently verified.

Validation:four complete workspaces 1421/0/1; native debug/release CLI,
runtime ABI and49 providers per target; focused A/C/D/E, retained contracts,
diagnostic structure/registry/coverage, docs/examples/quickstart, strict L002,
bounded comptime/VM and runtime ASan. Exact pins, jobs and limits are in the report.

## Proposed release and package checks

Proposed tag for review: `v0.1.0-alpha.1`; maintainer chooses final identity.
Two packages use the canonical target names. Each contains compiler CLI,
native runtime,49 canonical LLIB/object bundles, protocol/target/source/build
manifest, checksums, license and usage instructions. Document LLVM18 and GNU
toolchain prerequisites explicitly.

Build from the selected verified release commit. Smoke each downloaded package
in a separate clean Windows/Ubuntu job without a source checkout or external
sysroot: verify identities/hashes, then build/run the public quickstart and
a stdlib provider case using the package runtime/sysroot. Preserve all evidence.

Before tagging, prove release code-tree identity against the qualified pin;
requalify if implementation or artifact identity changes. Reconcile tag,
assets/checksums and release notes, then write the final closing record.

## Remaining authority and work

The inherited plan reserves PR/main merge, tag and publication to the maintainer
after this concrete handoff. None has been performed. Package acquisition and
clean-package smoke still need execution; qualification artifacts are evidence,
not distribution packages.
