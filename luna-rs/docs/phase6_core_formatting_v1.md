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
- `std::fmt::Display` implementations for supported values

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
`Result`; formatting implementations propagate it. Formatting is not
transactional: if a writer accepts some characters and later fails, those
accepted characters remain written, and the formatter stops at the first
failure. The `String` sink appends characters through its existing UTF-8
encoder.

`Display::fmt` is a generic trait method over `W: std::fmt::Writer`. Structured
user types can implement it using ordinary Luna trait syntax and write their
fields and separators to the provided sink.

## v1 implementations

The implementation covers `bool`, `char`, owned `std::String`, all Luna
integer types: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`,
`u64`, `u128`, and `usize`, plus the existing `std::FileError` enum. Integer
formatting is decimal. Signed formatting uses a minimum-safe magnitude
calculation rather than directly negating the minimum value. The algorithm is
allocation-free in `core/fmt` and writes digits in order. The current optional
`FileError` implementation proposes the messages `file not found`,
`permission denied`, `invalid input`, and `I/O error` for its four variants.
These exact strings are not frozen and remain subject to design/freeze review.

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

## Existing implementation and previously recorded evidence

The pre-existing permanent Luna fixtures exercise custom structured `Display`,
generic formatting into `String`, bools, chars, Unicode String contents,
representative integer values, integer boundaries, `u128::MAX`, all `FileError`
variants, and writer failure after a prefix. The prior acceptance matrix was
narrower than the complete v1 contract: it did not cover every integer edge,
multiple failure positions, or fresh source/artifact behavior for all negative
cases. The expanded fixtures and harnesses now cover those gaps on Windows;
native Linux/macOS execution and final full-workspace verification remain
pending.

The updated positive matrix now includes seven fixture programs in both
source-only and freshly built artifact-only sysroots, including all integer
types at zero/positive/minimum/maximum boundaries, Unicode scalar-width
boundaries and embedded NUL, a second generic Writer implementation, writer
failure at first/middle/final positions, and all proposed `FileError` texts.
Four negative fixtures are checked in both provider modes. The CLI harness also
executes all seven positive fixtures.

Windows focused evidence on the current worktree:

- Generic trait-bound validation (`Supported` implemented for `i32`, absent
  for `[u8; 1]`): **1/1 passed**.
- Integer cast signedness regression: **1/1 passed**.
- Enum tag/codegen regression: **1/1 passed**.
- Expanded formatting source/fresh-artifact acceptance: **2/2 passed**.
- Sysroot invariants: **5/5 passed**.
- Phase 5 whole-file I/O regressions: **3/3 passed**.
- Public CLI formatting E2E, after rebuilding current sysroot artifacts:
  **1/1 passed**.

These are local Windows results only. The full command
`cargo test --workspace -- --test-threads=1` has since completed on Windows
with exit code 0 on the current worktree. Native Linux/macOS workflow evidence
for the final commit is still required.

Earlier evidence collected 2026-09-30 on Windows (for the then-current,
narrower matrix):

- `stdlib_format_acceptance_tests`: 2/2 passed for the earlier four-positive-
  fixture and four-negative-case matrix.
- `stdlib_format_cli_acceptance`: 1/1 passed for the earlier three positive
  fixtures.
- Sysroot invariants: 5/5 passed; canonical baseline was 32 providers / 91
  direct dependency edges.
- The earlier full-workspace run exited 0 for the code present at that time;
  it does not verify the expanded worktree matrix now under review.

Full-range `u128` and the signed minimum values are tested using
shift/arithmetic construction, avoiding oversized positive literals.
Investigation also found a separate numeric-literal issue: the expression
`-9223372036854775808 as i64` currently compiles to zero, while constructing
the same `i64::MIN` value with in-range arithmetic works. The minimal source
reproducer is `../tests/luna/compiler/numeric_negative_i64_min_literal.ln`.
This discrepancy is outside the formatter implementation; its exact integer
literal contract needs compiler-language review before any compiler change.
The formatter boundary acceptance therefore uses the valid arithmetic
construction and does not treat the incorrect literal result as expected
behavior.

## Phase status dependencies

The `fmt` provider and its initial `String` sink changed the active sysroot
baseline to 32 providers and 90 direct dependency edges. `file` now depends on
`fmt` for `FileError` formatting, bringing the current baseline to 32
providers / 91 direct edges. Both values are asserted by the sysroot DAG
invariant test.

Phase 5 has native Linux/macOS build/runtime evidence. Its generic enum-pattern
lowering blocker was fixed and merged to `main`; see the
[re-audit and resolution record](phase5_reaudit_2026_09_30.md). Phase 5 remains
NOT FROZEN pending design-authority review.

Phase 6 remains NOT FROZEN pending design-authority review and the updated
acceptance, source/artifact parity, and regression results below.

## Generic compiler defect found during Phase 6 validation

The new fieldless-enum formatting case exposed a generic lowering defect:
resolved enum-variant identifiers in match arms were treated as irrefutable
identifier patterns during MVIR generation. The generator now distinguishes
resolved `SymbolKind::EnumVariant` patterns and emits a tag comparison. All
`Tag` instructions (match, `?`, and for-in protocol paths) now carry semantic
`u32` type metadata matching the tag value width. A permanent independent Luna
fixture covers four fieldless variants and one payload variant in
`tests/luna/compiler/enum_tag_match_codegen.ln`. This compiler change is generic
and not specific to `FileError` or formatting. No C-GAP ID was assigned; the
repository still has no canonical compiler-gap registry.

The expanded integer edge acceptance also exposed a generic backend
defect: LLVM's signless integer cast helper sign-extended unsigned values when
an explicit `as` conversion widened them (for example, `255 as u8 as u64`
became `u64::MAX`). Integer-to-integer cast lowering now supplies the source
type's signedness explicitly, and the permanent independent regression is
`tests/luna/compiler/integer_cast_signedness.ln`. The generic executable
regression passes on Windows; native Linux/macOS execution and final workspace
verification remain pending.

The expanded negative matrix exposed a generic typechecker defect as well:
`check_bounds_for_call` skipped trait obligations for concrete semantic types
without a primitive/nominal impl key. The independent `Supported` trait case
showed an array without an impl incorrectly passed. Bound checking now also
examines applicable trait-impl patterns when no such key exists. The permanent
reproducer and positive primitive control are
`tests/luna/compiler/generic_trait_bound_array_rejects.ln` and
`generic_trait_bound_primitive_accepts.ln`; both the compiler regression and
formatting negatives pass on Windows. Diagnostic parity normalizes session-
local `SemanticTypeId` numbers only, while still requiring the expected
semantic error category. The Phase 6 status remains IN PROGRESS.
