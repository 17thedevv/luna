# Stage 7 — Phase 6 completion plan

Date: 2026-09-30. This is an implementation/verification checklist, not freeze
approval.

## Current ground truth

- Phase 5 Whole-File I/O v1 is implementation-complete and cross-platform
  verified on commit `3dac3ac0e85204411bd80fac15b3294dd812e1be` (merged to
  `main`); it remains NOT FROZEN pending design-authority review. See
  [the Phase 5 re-audit and closure](phase5_reaudit_2026_09_30.md).
- Phase 6 is **Core Formatting Foundation v1**. The committed `core/fmt`
  trait-based implementation and String sink are the foundation; preserve
  their generic design and do not add compiler or runtime formatting magic.
- Current Phase 6 work is being developed on a branch based on the merged
  Phase 5 commit, separate from changes in the user's original worktree.
- A separate numeric-literal behavior (`-9223372036854775808 as i64` currently
  evaluates incorrectly) remains under investigation. No authoritative source
  found so far defines the intended signed-minimum literal interpretation;
  do not change semantics without design authority or an established contract.

## Contract to validate

- Public API under `std::fmt`: `Writer`, `Display`, and `FmtError::WriteFailed`.
- `Writer::write_char` accepts one Unicode scalar; it never accepts arbitrary
  bytes. `String` remains valid UTF-8; `Vec<u8>` remains arbitrary bytes.
- `Display::fmt` is an ordinary generic trait method over `W: Writer`.
- `core/fmt` remains allocation-free and depends only on `result`; the
  `String` sink lives in `alloc/string`.
- On write failure, the accepted prefix remains, formatting stops at the first
  error, and no later characters are attempted. No rollback or recoverable-OOM
  promise is added.
- `FileError: Display` and its exact lowercase messages are an optional Phase 6
  extension under review; they are not frozen API text.

## Remaining implementation and acceptance work

1. Implement the proposed `FileError` Display through ordinary Luna trait
   dispatch; rebuild artifacts from source and include all variants in both
   CLI and source/artifact tests.
2. Exercise all 12 integer widths at zero, signed/unsigned boundaries, decimal
   powers and neighbors, and full `u128`/`i128` values constructed without
   depending on the unresolved oversized-negative-literal behavior.
3. Exercise bool, ASCII, Unicode scalar boundaries, multi-byte Strings, empty
   Strings, embedded NUL text, appending to nonempty String, structured user
   `Display`, and at least two successful Writer types.
4. Test write failures at first, middle, and final character; assert accepted
   prefix, exact `WriteFailed`, attempt count, and immediate stop.
5. Verify negative cases for byte arrays, `f32`, `f64`, and legacy root
   `Display` reject for the trait/visibility reason, in both provider modes.
6. Execute positive fixtures through public `luna build`; compare exact
   outputs, exit codes, and relevant diagnostics between source-only providers
   and freshly built `.llib/.obj`-only providers with no source fallback.
7. Re-derive the sysroot count after `file -> fmt`; current expected baseline is
   32 providers / 91 direct edges. Assert it in the invariant suite.
8. Run focused compiler regressions, formatting CLI and parity, Phase 5 file
   regressions, namespace/sysroot invariants, then
   `cargo test --workspace -- --test-threads=1`; record commit, platform, and
   exit status. Runtime ABI need not rerun unless runtime/ABI code changes.
9. Correct all evidence docs from actual final runs. Do not claim
   IMPLEMENTATION COMPLETE until all selected gates pass. Do not claim FROZEN;
   leave design/freeze approval to the maintainer.

## Scope exclusions and stop conditions

No Debug, float formatting, macros/parser/format strings, printf syntax,
locale, width/alignment/precision, file or streaming Writer, logging,
environment/time/thread APIs, runtime formatting code, or namespace aliases.
Stop for ambiguous language semantics, source/artifact divergence, unsafe
ownership changes, or any proposed formatting-specific compiler branch.
