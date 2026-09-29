# Stage 7 — Phase 6: Core Formatting Foundation v1

Status: IMPLEMENTATION COMPLETE — READY FOR DESIGN/FREEZE REVIEW — NOT FROZEN

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

The permanent Luna fixture exercises a custom structured `Display` impl,
generic formatting into `String`, bools, chars, Unicode String contents, and
representative values for every integer type. A negative fixture confirms that
arbitrary byte arrays and floats do not accidentally implement `Display`.
The same full-codegen fixture runs with source-only providers and freshly built
artifact-only providers; exit code and stdout must match exactly.

Evidence collected 2026-09-29 on Windows:

- `stdlib_format_acceptance_tests`: 2/2 pass, including exact expected output,
  error propagation, negative byte/float trait-selection checks, and source/
  fresh-artifact parity.
- Sysroot invariants: 5/5 pass; canonical baseline is 32 providers / 90 direct
  dependency edges.
- `cargo test --workspace -- --test-threads=1`: exit code 0, including
  doc-tests.

The current compiler's numeric literal/backend support limits direct testing of
the full `u128` numeric range. The formatter implementation handles the type,
but an acceptance claim for extreme `u128` values remains pending a separately
supported literal/construction path.

## Phase status dependencies

Adding the `fmt` provider changes the active sysroot baseline to 32 providers
and 90 direct dependency edges. Both values are asserted by the sysroot DAG
invariant test.

Phase 5 Whole-File I/O remains implementation-complete but not frozen while
native POSIX build/runtime evidence is unavailable. Phase 6 formatting does
not consume the POSIX file-I/O contract and does not change its status.

Phase 6 remains NOT FROZEN pending design-authority review. The full workspace
and source versus fresh `.llib` gates currently pass; extreme `u128` boundary
formatting remains an explicit test limitation described above.
