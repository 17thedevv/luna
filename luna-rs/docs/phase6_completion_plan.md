# Stage 7 — Phase 6 closure / design-review plan

Updated: 2026-10-01. NOT FROZEN; freeze authority remains with the maintainer.

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

Current operator/CLI follow-up verification is in progress:

- CLI formatting parity: eight positive, four negative fixtures in each mode.
- Char operators: 32 negative fixtures, check + build in each mode; controls.
- Existing char casts: two positive, ten compile-negative, six runtime-abort
  fixtures in each mode.
- Generic trait-bound, integer cast signedness, enum tag/codegen regressions.
- Sysroot invariants: 32 providers / 91 direct DAG edges.
- Phase 5 whole-file regressions as a frozen-contract dependency.
- Windows full workspace, actual exit code.
- Native Ubuntu/macOS formatting + full Ubuntu workspace on the exact follow-up
  implementation commit.
- `git diff --check`.

Runtime and ABI are untouched. Do not rerun ABI merely to imply a broader
runtime change. Repository-wide formatting is not clean; do not claim it is.

## Separate known compiler issues

`numeric_negative_i64_min_literal.ln` documents the independently approved
signed-minimum literal issue; it is not a passing semantic regression.

`numeric_compound_assignment_diagnostic.ln` documents a newly observed
pre-existing lowering defect: MVIR ignores AssignOp and treats compound
assignment as ordinary assignment. Its correct oracle currently exits 1.
Formatting v1 does not use compound assignment. Do not endorse this behavior,
change the formatting contract, or silently fix it as part of Phase 6.

## Handoff

When all gates above pass, report:

PHASE 6 IMPLEMENTATION COMPLETE
READY FOR DESIGN/FREEZE REVIEW
NOT FROZEN

Do not merge main, self-freeze, start another phase, or broaden the public API.
The canonical contract and exact evidence are in
[phase6_core_formatting_v1.md](phase6_core_formatting_v1.md).
