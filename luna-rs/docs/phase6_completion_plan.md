# Stage 7 — Phase 6 completion plan

Date: 2026-09-30. Implementation evidence is recorded below; this is not
freeze approval.

## Ground truth and ordering

Phase 6 is **Core Formatting Foundation v1**, not environment, time,
filesystem streaming, or a new formatting language. `core/fmt` and the String
sink already exist in committed source; do not rebuild them from scratch.
Uncommitted work adds FileError formatting, broader acceptance, and a generic
enum-lowering correction. Preserve and audit that work rather than mixing it
into the POSIX merge.

Native Linux/macOS Phase 5 CI and the full Ubuntu workspace gate are green on
`3dac3ac0e85204411bd80fac15b3294dd812e1be`, now merged to `main`. The
[Phase 5 re-audit](phase5_reaudit_2026_09_30.md) records the enum-lowering
defect and its closure. Phase 5 is implementation-complete and
cross-platform-verified, but remains NOT FROZEN pending design-authority
review. Phase 6 work is committed separately on
`codex/phase6-formatting-closure`.

## 0. Close Phase 5 error behavior — COMPLETE

1. Audit the existing uncommitted generic enum fix. Distinguish resolved
   enum variants from variable bindings at both top-level and nested pattern
   positions. Verify tag metadata has the correct integer semantic type.
   Use resolved symbol identity, not variant/provider-name suffix searches.
2. Add permanent executable harnesses for the two new audit fixtures and
   the existing `enum_tag_match_codegen.ln`. Cover fieldless/payload variants,
   nested enums, actual bindings/wildcards, multiple enums with colliding
   terminal variant names, and generic enum instances. Include success and
   deliberately non-matching controls.
3. Expand Whole-File I/O acceptance: failed destination write must propagate
   through `copy_file`; missing source must not modify destination; same-path
   and embedded-NUL errors must have the exact variant. Prove each classifier
   rejects other variants before using it as an oracle. Cover deterministic
   I/O failures and permission errors where the runner can reliably induce
   them; do not count a skipped permission case as verified.
4. Run public CLI E2E and fresh source-only/artifact-only fixtures with no
   fallback. Then run affected match/enum, `?`, for-in, namespace, sysroot,
   file-I/O, and workspace regressions. Record exact commit and exit status.
5. Re-run Ubuntu/macOS Whole-File I/O CI with the stronger tests. Preserve
   the established runtime ABI/ASan coverage; do not reopen runtime semantics
   absent a demonstrated runtime defect.
6. Keep the compiler repair/regression commit separate from Phase 6 APIs.
   After all gates pass, fetch `main`, review the exact commit set and merge
   without force or uncommitted Phase 6 changes. Preserve the user's worktree.

Closure evidence: the nested/duplicate-name enum reproducers pass; failed
destination writes propagate through `copy_file` in source and fresh
artifacts; POSIX ABI, parity, ASan cleanup, and full Ubuntu workspace all pass;
`main` is at the reviewed closure commit. Phase 5 is ready for
design/freeze review, not self-frozen.

## 1. Reconcile Phase 6 contract and existing implementation

- Canonical API: `std::fmt::Writer`, `std::fmt::Display`,
  `std::fmt::FmtError::WriteFailed`.
- Writer accepts one Unicode scalar via `write_char`, not arbitrary bytes.
  String stays valid UTF-8; byte vectors do not gain textual semantics.
- Display is an ordinary trait with generic `fmt<W: Writer>` dispatch.
- `core/fmt` remains allocation-free and depends on `result`, not `alloc`.
  `alloc/string` supplies Writer and Display for String.
- Failure is non-transactional: accepted prefix remains; stop at first error;
  no later writes. No rollback or recoverable-OOM promise.
- Reconcile the FileError extension and exact messages with design authority
  before treating them as frozen API text.

Deliverable: one consistent contract document with evidence separated from
claims. Remove the stale assertion that native POSIX evidence is unavailable.

## 2. Isolate numeric-literal debt without masking it

The current Phase 6 record reports that `-9223372036854775808 as i64` becomes
zero. Audit lexer/parser/type inference/comptime/MVIR/backend against the
existing integer-literal contract. Compare decimal, suffixed, cast, and
arithmetic constructions and neighboring boundaries.

The existing diagnostic fixture returns 0 for the known bad zero result. It
must not become a permanent acceptance test with that expectation. After a
generic repair, replace the oracle with the intended numeric value and add
compile-fail checks for genuinely invalid literals. If the language contract
is ambiguous, stop for design authority rather than changing semantics.

Keep integer formatting tests on independently verified input values. Do not
hide a compiler defect by relaxing formatter output expectations. Any decision
to carry a separate known literal defect into Phase 6 freeze needs explicit
maintainer approval.

During the expanded integer edge run, a distinct generic backend issue was
observed: widening casts such as `255 as u8 as u64` sign-extended instead of
zero-extending. The backend repair uses the source type's semantic signedness,
with an independent compiler fixture at
`tests/luna/compiler/integer_cast_signedness.ln`. This is not the separate
`-9223372036854775808` literal-parsing issue. The focused Windows regression,
native Ubuntu/macOS formatting acceptance, and Ubuntu full-workspace gate all
pass on Phase 6 commit `dc123dfb0bd8748b0dcb259fc779b3d2f9da3c7c`.

The expanded negative matrix also exposed a generic typechecker gap: call-site
trait bounds were checked only when the concrete argument had a primitive or
nominal `ImplSelfTypeKey`. Other concrete semantic types, including fixed-size
arrays, skipped the bound check. A user-defined `Supported` trait reproducer
confirmed that `i32` with an impl passed and `[u8; 1]` without an impl also
passed. The checker now validates trait-impl patterns even when no such key is
available. Its permanent compiler regression is
`tests/luna/compiler/generic_trait_bound_array_rejects.ln`, with a positive
primitive control in `generic_trait_bound_primitive_accepts.ln`. No formatter,
provider, type-name, or trait-name special case was added.

Source/artifact negative diagnostics can contain session-local
`SemanticTypeId` values. The parity harness normalizes only these ephemeral
numeric IDs before comparing messages; it still asserts the expected semantic
failure reason and identical normalized diagnostics.

The complete Windows workspace command
`cargo test --workspace -- --test-threads=1` completed with exit code 0 after
the generic trait-bound repair. Phase 6 workflow run
[36694791949](https://github.com/17thedevv/luna/actions/runs/36694791949)
passed its Ubuntu 24.04 and macOS 15 Intel formatting acceptance jobs and the
full Ubuntu workspace regression. These checks ran against commit
`dc123dfb0bd8748b0dcb259fc779b3d2f9da3c7c`.

## 3. Complete the formatting acceptance matrix

Reuse the existing implementation and four positive fixtures, then fill gaps:

- Decimal output for all 12 integer types: zero, positive/negative values,
  MIN/MAX, powers of ten and adjacent values, u128/i128 extremes.
- Bool, ASCII, Unicode scalar boundaries and multi-byte String contents,
  empty text, embedded NUL as valid text, and appending to a nonempty String.
- User-defined structured Display; at least two different Writer types and
  multiple generic instantiations through ordinary trait dispatch.
- Failing writer at positions zero, middle, and final character; verify
  accepted prefix, attempt count, exact error, and no subsequent write.
- All FileError variants if the extension remains in scope; do not rely on
  the defective enum matcher as the sole oracle.
- Negative cases for floats, arbitrary bytes, and legacy root Display must
  reject for the intended trait/visibility reason, not a syntax error.

The current CLI harness runs three fixtures but omits FileError; include all
positive fixtures through `luna build` and executable output verification.

## 4. Fresh artifacts, layering, and platform checks

- Rebuild canonical artifacts with the final compiler and providers.
- Execute identical positive fixtures in source-only and `.llib/.obj`-only
  roots, removing source availability in the latter. Compare exact output,
  exit code, and relevant diagnostics, including negative cases.
- Verify exported traits, method generics and impls survive fresh sessions;
  no format-specific compiler hook or portable session ID is introduced.
- Re-derive provider/DAG counts. Committed baseline is 32 providers / 90
  direct edges; FileError's proposed `file -> fmt` dependency yields 32 / 91.
  Assert the selected final graph, not an outdated 31 / 87 baseline.
- Run Windows and native Linux/macOS formatting E2E, particularly 128-bit
  formatting and generic trait codegen. Do not infer platform coverage from
  stdlib-only source changes.

## 5. Regression, review, and commit boundaries

Run in this order: compiler regressions (integer casts, generic trait bounds,
enum patterns); formatting CLI; formatting source/artifact matrix; Phase 5
regression; namespace/sysroot invariants;
`cargo test --workspace -- --test-threads=1`; `git diff --check`.

Do not rerun runtime ABI merely because formatting changed; retain native
Phase 5 evidence, and rerun ABI if runtime/ABI-sensitive behavior changes.
Avoid repository-wide formatting and unrelated `luna-web` or generated-MVIR
changes. Audit any generated output before deciding it belongs in a commit.

Suggested independent commits:

1. Phase 5 generic enum repair + independent regressions + error closure.
2. Phase 6 generic integer-cast signedness repair + compiler regression.
3. Phase 6 stdlib extension + canonical contract changes.
4. Formatting CLI/parity/adversarial acceptance and final evidence.

Every evidence record names its commit, platform, command and exit status.
Do not reuse an earlier workspace run to certify later compiler changes.

Completion report: **PHASE 6 IMPLEMENTATION COMPLETE — READY FOR
DESIGN/FREEZE REVIEW — NOT FROZEN**. Acceptance, native workflow, Windows
workspace, and CI workspace gates now pass. Design authority must still review
the separately documented negative-i64-literal compiler issue and proposed
FileError display text.

## Non-goals and stop conditions

No Debug, float formatting, macros, format-string parser, locale,
width/alignment/precision, file/stream Writer, logging, env/time/thread APIs,
runtime formatting functions, or namespace aliases.

Stop for a missing semantic decision, provenance/ownership contradiction,
unresolved source/artifact divergence, or a proposed stdlib-name compiler
special case. Do not broaden the phase to make a test pass.
