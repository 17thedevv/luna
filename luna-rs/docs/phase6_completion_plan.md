# Stage 7 — Phase 6 closure / freeze record

Updated: 2026-10-01. RESOLVED & FROZEN by the human maintainer's explicit
approval: "Phê duyệt freeze Phase 6 trên baseline 0394776".
Verified implementation: `039477650b8cd0b39d058c153c96e3253f02fc75`.

## Scope and decisions

Phase 6 is Core Formatting Foundation v1: ordinary `std::fmt::Writer`,
`std::fmt::Display`, `std::fmt::FmtError::WriteFailed`, allocation-free decimal
integer formatting, bool/char/String formatting, user-defined structured
Display and owned String as a textual sink.

Approved decisions:

- FileError public messages are `file not found`, `permission denied`,
  `invalid input`, `I/O error` (2026-09-30).
- The signed-minimum numeric-literal defect is tracked outside Phase 6
  (2026-09-30).
- Reject arithmetic/bitwise/shift and compound assignment on char; retain
  comparison, ordinary assignment and explicit integer casts (2026-10-01).

No Debug, floats, formatting macros/parser, locale, width/alignment/precision,
file/stream Writer, logging, env/time/thread or runtime formatting APIs.

## Prerequisite and historical evidence

Phase 5 implementation is merged to main at
`3dac3ac0e85204411bd80fac15b3294dd812e1be`, with native Ubuntu/macOS
evidence in [36670078044](https://github.com/17thedevv/luna/actions/runs/36670078044).
It remains separately NOT FROZEN pending design review.

Phase 6 native CI passed on `dc123df` in
[36694791949](https://github.com/17thedevv/luna/actions/runs/36694791949)
and on `765b910` in
[36800606504](https://github.com/17thedevv/luna/actions/runs/36800606504).
The latter closes integer-to-char cast validation, but does **not** validate
the subsequent operator restriction and CLI-only parity migration.

The Windows full run on 765b910 ran out of C: disk space and exited 1.
It is not a passing Windows workspace gate. Current verification redirects
temporary artifacts to D: and captures the actual process exit status.

## Current implementation closure

1. Shared primitive operator restrictions in semantic type checking.
2. Revalidation after generic substitution, including imported artifact bodies.
3. No String/Display/provider-specific compiler branches or new lang items.
4. Standalone compiler fixtures and user-defined generic provider controls.
5. Full formatting acceptance/negative parity through actual CLI commands.
6. Clean fresh sysroot build, source-only / artifact-only roots, no fallback.
7. Strict UTF-8 + exact output / exit status oracle.
8. Writer failure tested at every character boundary for 21 values.
9. Scoped formatting only; preserve unrelated dirty/generated work.

The obsolete internal driver formatting harness is removed, with equivalent
and stronger coverage owned by `stdlib_format_cli_acceptance`. Native CI uses
that public harness and the independent char-operator suite.

## Final gates

All operator/CLI follow-up gates below pass on the pinned implementation:

- CLI formatting parity: eight positive, four negative fixtures in each mode.
- Char operators: 32 negative fixtures, check + build in each mode; controls.
- Existing char casts: two positive, ten compile-negative, six runtime-abort
  fixtures in each mode.
- Generic trait-bound, integer cast signedness, enum tag/codegen regressions.
- Sysroot invariants: 32 providers / 91 direct DAG edges.
- Phase 5 whole-file regressions as an implementation dependency; this does not
  declare Phase 5 frozen.
- Windows full workspace, actual exit code.
- Native Ubuntu/macOS formatting + full Ubuntu workspace on the exact follow-up
  implementation commit.
- `git diff --check`.

### Current exact-commit evidence

Implementation commit: `039477650b8cd0b39d058c153c96e3253f02fc75`.
Parallel, uncommitted stdlib work is excluded from this review baseline.

| Gate | Result |
| --- | --- |
| Windows isolated formatting / char-scalar / char-operator CLI | 2/2, 1/1, 2/2; exit 0 |
| Windows operator / trait-bound / integer-cast / enum-tag regression | 1/1 each; exit 0 |
| Windows isolated sysroot / whole-file regression | 5/5, 3/3; exit 0 |
| Windows isolated full workspace | exit 0; 1,222 passed, 0 failed, 1 pre-existing ignored |
| Ubuntu and macOS native formatting / scalar / operator CLI | 2/2, 1/1, 2/2 on each platform; success |
| Ubuntu native full workspace | exit 0; 1,222 passed, 0 failed, 1 pre-existing ignored |
| Native job whitespace checks | all three jobs passed |
| Windows isolated / Phase 6 documentation whitespace checks | exit 0 |

Native evidence is
[36814704447](https://github.com/17thedevv/luna/actions/runs/36814704447),
with every job targeting exactly 0394776. No macOS full-workspace claim is made.
Windows verification uses a separate managed checkout, Cargo target and fresh
sysroot. Its full run completed with actual exit code 0 on 2026-10-01. The
earlier main-worktree run exited 101 during concurrent source/artifact changes
and is not counted as a passing gate. All 16 String acceptance tests now pass
on the isolated commit. See the canonical record for details.

Runtime and ABI are untouched by this pinned Phase 6 implementation. Do not
rerun ABI merely to imply a broader runtime change. Parallel runtime additions
are outside this freeze. Existing compiler warnings and repository-wide
formatting debt remain; do not claim warning-free or globally clean status.

## Separate known compiler issues

`numeric_negative_i64_min_literal.ln` documents the independently approved
signed-minimum literal issue; it is not a passing semantic regression.

`numeric_compound_assignment_diagnostic.ln` documents a newly observed
pre-existing lowering defect: MVIR ignores AssignOp and treats compound
assignment as ordinary assignment. Its correct oracle currently exits 1.
Formatting v1 does not use compound assignment. The maintainer explicitly
approved retaining both recorded numeric defects outside the freeze scope.
Do not endorse their behavior, change the formatting contract, or silently fix
them as part of Phase 6.

## Handoff

All gates above passed. Current handoff status:

PHASE 6 IMPLEMENTATION COMPLETE
CORE FORMATTING FOUNDATION v1
RESOLVED & FROZEN
2026-10-01 — implementation baseline 0394776

The freeze is maintainer-authorized, not agent self-certification. Do not merge
main, start another phase or broaden the public API under this authorization.
The canonical contract and exact evidence are in
[phase6_core_formatting_v1.md](phase6_core_formatting_v1.md).

The maintainer's requested review of Antigravity's subsequent stdlib additions
remains separate; those parallel changes are not included in this closure.
SKILL IMPACT: none; the existing public CLI/parity workflow remains applicable.
