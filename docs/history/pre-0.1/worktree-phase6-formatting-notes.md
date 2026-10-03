<!-- luna-doc-role: historical -->

> Historical phase5-audit worktree notes preserved during 2026-10-03 integration. The [versioned specification](../../spec/0.1/README.md) and newer exact-commit formatting record govern current contracts.

# Stage 7 — Phase 6: Core Formatting Foundation v1

Status: IMPLEMENTATION IN PROGRESS — NOT FROZEN

## Boundary

Formatting is a stdlib capability built on ordinary traits. The compiler has no
format-specific syntax, hook, builtin method lookup, or formatting opcode.
Formatting code is expected to compile and link through the same generic trait
dispatch, `.llib` metadata, and object-artifact paths as other stdlib code.

The public logical API is under `std::`:

- `std::fmt::Writer`
- `std::fmt::Display`
- `std::fmt::FmtError`
- `std::Display` implementations for supported values

The core `fmt` provider must not depend on allocation. `alloc/string` supplies
the first owned sink by implementing `Writer for std::String` and `Display for
std::String`.

## v1 writer contract

`Writer::write_char(&rw Self, char) -> std::Result<void, FmtError>` appends one
Unicode scalar value. It deliberately accepts characters rather than
arbitrary bytes, so textual output remains valid UTF-8 and cannot be confused
with a `Vec<u8>` containing arbitrary binary data. v1 has no byte-slice writer,
file writer, streaming I/O writer, or fixed-buffer implementation.

`FmtError` currently contains `WriteFailed`. A writer reports failure through
`Result`; formatting implementations propagate it. The `String` sink appends
characters through its existing UTF-8 encoder.

`Display::fmt` is a generic trait method over `W: std::fmt::Writer`. Structured
user types can implement it using ordinary Luna trait syntax and write their
fields and separators to the provided sink.

## v1 implementations

The implementation covers `bool`, `char`, owned `std::String`, and all Luna
integer types: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`,
`u64`, `u128`, and `usize`. Integer formatting is decimal. Signed formatting
uses a minimum-safe magnitude calculation rather than directly negating the
minimum value. The algorithm is allocation-free in `core/fmt` and writes
digits in order.

The current working change proposes `Display for std::FileError` using the
messages `file not found`, `permission denied`, `invalid input`, and `I/O
error`. This is an optional Phase 6 extension; the exact text remains subject
to design-authority review and is not a frozen compatibility promise.

String formatting decodes the String's already-valid UTF-8 into Unicode scalar
values and sends those to the writer. It does not reinterpret each UTF-8 byte
as a character and does not accept arbitrary byte vectors.

## Explicitly out of scope

- float formatting (deferred with CORE-GAP-18)
- `Debug`
- formatting macros and format-string parsing
- printf-style formatting
- width, alignment, precision, locale, or escape mini-languages
- formatting to files, streams, or a general I/O `Write` abstraction
- logging
- compiler special-cases or runtime formatting support

## Acceptance evidence

The permanent Luna fixtures cover structured `Display`, generic formatting
into both `String` and a user-defined successful writer, all integer widths,
integer boundaries, Unicode scalar encoding, embedded NUL text, empty strings,
writer failures at first/middle/final positions, and all `FileError` variants.
Negative cases cover arbitrary byte arrays, both float widths, and the legacy
root `Display` spelling. Current changes are still being validated; the
coverage list is not itself evidence that those tests pass.

Previously recorded baseline evidence collected 2026-09-29 on Windows, before
the current expanded acceptance changes:

- `stdlib_format_acceptance_tests`: 2/2 pass for the original formatting
  fixture, error propagation, negative byte/float trait-selection checks, and
  source/fresh-artifact parity.
- Sysroot invariants: 5/5 pass; canonical baseline is 32 providers / 90 direct
  dependency edges.
- `cargo test --workspace -- --test-threads=1`: exit code 0, including
  doc-tests.

This baseline does not verify the new FileError implementation or the expanded
acceptance matrix below. Do not reuse it as closure evidence for the current
worktree.

## Phase status dependencies

Adding the `fmt` provider changes the active sysroot baseline to 32 providers
and 90 direct dependency edges. The proposed `file -> fmt` dependency for the
FileError implementation adds one direct edge, so the current intended
baseline is 32 providers and 91 direct edges. Both counts must be re-derived
and asserted after rebuilding the canonical artifacts.

Phase 5 Whole-File I/O has native Linux/macOS build/runtime, source/artifact,
ASan cleanup, and full Ubuntu workspace evidence on commit
`3dac3ac0e85204411bd80fac15b3294dd812e1be`, merged to `main`. It is
implementation-complete and cross-platform-verified, but remains NOT FROZEN
pending design-authority review. The Phase 5 re-audit and its resolution are
recorded in `phase5_reaudit_2026_09_30.md`.

The expression `-9223372036854775808 as i64` has a known incorrect result in the
current compiler. Existing code defaults unsuffixed integer literals to `i32`,
but the language contract for the minimum signed literal is not explicit. No
compiler semantics have been changed. Phase 6 acceptance uses independently
constructed values; carrying this separate literal issue into a future freeze
requires design-authority acceptance or a generic compiler repair under an
established contract.

Source tracing narrows the likely mechanism: the parser consumes unary `-`
inside `parse_unary` before the trailing `as` cast is built; type checking
assigns the positive integer token `i32`; MVIR preserves the token's decimal
text; and backend lowering materializes ordinary numeric operands as `i32`.
The backend has a separate direct-literal cast path, but this expression's
outer cast applies after unary negation. That matches the reported truncation
shape, but does not decide whether the language should accept this spelling or
reject the out-of-range default-`i32` operand. The compiler behavior therefore
remains unmodified pending contract/design review.

Phase 6 remains NOT FROZEN. The expanded source/fresh-artifact and public CLI
tests, current provider graph, and full regression still need to pass on the
final implementation commit before reporting implementation completion.

## Current branch evidence — 2026-09-30

The expanded Rust acceptance harnesses compile on Windows with
`cargo test -p luna-driver --test stdlib_format_acceptance_tests --no-run`
and `cargo test -p luna-cli --test stdlib_format_cli_acceptance --no-run`
(both exit 0). Executing them is currently unavailable on this host:
the driver test process exits with Windows status `0xC0000135`
(`STATUS_DLL_NOT_FOUND`), and the CLI harness's child compiler fails the same
way because `LLVM-C.dll` is not installed. Scoped `rustfmt --check` for the
formatting harnesses and `git diff --check` pass. A dedicated native Ubuntu /
macOS formatting workflow has been added but has not run yet; these compile
checks are not behavioral or cross-platform evidence.
