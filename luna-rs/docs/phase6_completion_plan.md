# Stage 7 — Phase 6 design/freeze review plan

Date: 2026-09-30. Implementation evidence is recorded below; this is not
freeze approval.

## Ground truth and ordering

Phase 6 is **Core Formatting Foundation v1**, not environment, time,
filesystem streaming, or a new formatting language. The implementation,
acceptance expansion, and generic compiler repairs are committed on
`codex/phase6-formatting-closure`; do not treat this review plan as an
implementation backlog or merge it into the Phase 5 closure.

Native Linux/macOS Phase 5 CI and the full Ubuntu workspace gate are green on
`3dac3ac0e85204411bd80fac15b3294dd812e1be`, now merged to `main`. The
[Phase 5 re-audit](phase5_reaudit_2026_09_30.md) records the enum-lowering
defect and its closure. Phase 5 is implementation-complete and
cross-platform-verified, but remains NOT FROZEN pending design-authority
review. Phase 6 work is committed separately on
`codex/phase6-formatting-closure`.

## 0. Phase 5 prerequisite — CLOSED

Phase 5 Whole-File I/O v1 was re-audited, its generic enum-pattern lowering
defect and error-path tests were repaired, and closure commit
`3dac3ac0e85204411bd80fac15b3294dd812e1be` is already an ancestor of
`origin/main`. Native Ubuntu/macOS ABI and source/fresh-artifact gates, Ubuntu
ASan cleanup, and the full Ubuntu workspace passed in CI run
[36670078044](https://github.com/17thedevv/luna/actions/runs/36670078044).
Phase 5 is implementation-complete and cross-platform verified, but remains
NOT FROZEN pending design-authority review. No Phase 5 merge remains to do.

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
- The `FileError` Display implementation is present and covered. Design
  authority approved the four exact messages as public API text on
  2026-09-30: `file not found`, `permission denied`, `invalid input`, and
  `I/O error`.

The canonical contract and implementation evidence are recorded in
`phase6_core_formatting_v1.md`; design-authority choices are recorded below.
The remaining closure gate is the native Ubuntu full-workspace CI result.

## 2. Separate numeric-literal issue — tracked outside Phase 6

The expression `-9223372036854775808 as i64` currently compiles to zero, while
the same minimum value constructed using in-range arithmetic is verified by
the formatter acceptance tests. `LanguageReference.md` does not currently
define integer literal overflow or the special signed-minimum spelling; the
grammar parses unary minus separately from the positive integer token. The
permanent reproducer `tests/luna/compiler/numeric_negative_i64_min_literal.ln`
has a weak oracle that returns success when the value is zero, so it is
diagnostic evidence only, not a passing semantic regression.

Do not silently repair or freeze this behavior as part of formatting. Design
authority decided on 2026-09-30 to track this independent compiler issue
outside the Phase 6 contract. It does not block Phase 6 review. If the literal
semantics are addressed later, first define the contract and create a valid
assertion for `i64::MIN` plus compile-fail controls for out-of-range positive
literals.

Keep integer formatting tests on independently verified input values. Do not
hide a compiler defect by relaxing formatter output expectations. The explicit
maintainer decision to carry this separate issue outside Phase 6 is recorded
above.

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
completed successfully: both Ubuntu/macOS formatting acceptance jobs and the
full Ubuntu workspace regression concluded with success. It targets commit
`dc123dfb0bd8748b0dcb259fc779b3d2f9da3c7c`.

## 3. Formatting acceptance matrix — COMPLETE

The expanded acceptance is implemented and passing. Seven positive fixtures
cover:

- Decimal output for all 12 integer types: zero, positive/negative values,
  MIN/MAX, powers of ten and adjacent values, u128/i128 extremes.
- Bool, ASCII, Unicode scalar boundaries and multi-byte String contents,
  empty text, embedded NUL as valid text, and appending to a nonempty String.
- User-defined structured Display; at least two different Writer types and
  multiple generic instantiations through ordinary trait dispatch.
- Failing writer at positions zero, middle, and final character; verify
  accepted prefix, attempt count, exact error, and no subsequent write.
- All four FileError variants; the enum matcher defect has an independent
  compiler regression and is no longer the sole oracle.
- Four negative cases cover floats, arbitrary bytes, and legacy root Display
  rejection for the intended semantic/visibility reason.
- The public Luna CLI harness runs all seven positive fixtures through build,
  link, and execution, including `formatting_file_error_v1.ln`.

## 4. Fresh artifacts, layering, and platform checks — COMPLETE

Fresh canonical artifacts were rebuilt. Identical positive fixtures ran in
source-only and `.llib/.obj`-only roots with source unavailable in artifact
mode; outputs, exit codes, and normalized semantic diagnostics agree. Trait
exports, generic methods, and impls survive fresh sessions. No formatting
compiler hook or portable session ID was added.
- Current sysroot invariant is **32 providers / 91 direct edges**, verified
  by the passing 5/5 sysroot invariant suite.
Windows and native Ubuntu/macOS formatting E2E, including 128-bit formatting
and generic trait codegen, passed as recorded below and in the core contract.

## 5. Regression evidence and review actions

Evidence is recorded for Phase 6 implementation commit
`dc123dfb0bd8748b0dcb259fc779b3d2f9da3c7c` and documentation follow-up
`70568be7cde3d795c327c08b53c8c757b342919d`. Workflow run
[36694791949](https://github.com/17thedevv/luna/actions/runs/36694791949)
passed Ubuntu/macOS formatting acceptance and the full Ubuntu workspace
regression. Windows focused suites, public CLI acceptance, sysroot invariants,
Phase 5 acceptance, and Windows full workspace passed. Detailed local counts
are in `phase6_core_formatting_v1.md`.

Design-authority decisions are recorded: the four `FileError` display strings
are frozen public text, and the unrelated signed-minimum literal defect is
tracked outside Phase 6. The remaining freeze gate is the final result of the
native Ubuntu full-workspace CI job. No runtime ABI rerun is needed unless
runtime/ABI code changes. Repository-wide formatting is known to report broad
baseline differences and must not be claimed clean; do not reformat unrelated
files. Preserve the generated `test_model.mvir` and untracked `luna-web/`
worktree items.

Completion report: **PHASE 6 IMPLEMENTATION COMPLETE — READY FOR
DESIGN/FREEZE REVIEW — NOT FROZEN**. Local acceptance, Windows workspace,
native Ubuntu/macOS acceptance, and Ubuntu full-workspace gates pass.
Maintainer contract decisions are recorded above.

## Non-goals and stop conditions

No Debug, float formatting, macros, format-string parser, locale,
width/alignment/precision, file/stream Writer, logging, env/time/thread APIs,
runtime formatting functions, or namespace aliases.

Stop for a missing semantic decision, provenance/ownership contradiction,
unresolved source/artifact divergence, or a proposed stdlib-name compiler
special case. Do not broaden the phase to make a test pass.
