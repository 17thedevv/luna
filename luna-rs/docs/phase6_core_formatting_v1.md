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

## Unicode scalar validity

Luna `char` values are Unicode scalar values: `U+0000..U+D7FF` and
`U+E000..U+10FFFF`. Surrogates and values above `U+10FFFF` are not `char`
values, even though the backend represents `char` as a 32-bit integer.
Integer-to-`char` casts accept only integer source types. A statically known
invalid value is a compile-time diagnostic; a runtime-dependent invalid value
traps before a `char` is produced. Float, boolean, and pointer casts to `char`
are rejected. Safe formatter and String APIs therefore receive valid scalar
values; raw/foreign access remains subject to Luna's existing unsafe
obligations.

`Display::fmt` is a generic trait method over `W: std::fmt::Writer`. Structured
user types can implement it using ordinary Luna trait syntax and write their
fields and separators to the provided sink.

## v1 implementations

The implementation covers `bool`, `char`, owned `std::String`, all Luna
integer types: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`,
`u64`, `u128`, and `usize`, plus the existing `std::FileError` enum. Integer
formatting is decimal. Signed formatting uses a minimum-safe magnitude
calculation rather than directly negating the minimum value. The algorithm is
allocation-free in `core/fmt` and writes digits in order. Design authority
approved these exact public `FileError` messages on 2026-09-30: `file not
found`, `permission denied`, `invalid input`, and `I/O error`.

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
cases. The expanded fixtures and harnesses cover those gaps on Windows,
Ubuntu, and macOS as recorded below.

The updated positive matrix now includes seven fixture programs in both
source-only and freshly built artifact-only sysroots, including all integer
types at zero/positive/minimum/maximum boundaries, Unicode scalar-width
boundaries and embedded NUL, a second generic Writer implementation, writer
failure at first/middle/final positions, and all four approved `FileError`
texts.
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

### Follow-up correctness closure: Unicode scalar validity

A subsequent audit found that representing `char` as i32 was not enough to
guarantee Unicode scalar validity. Integer-to-char casts now validate the
Unicode scalar range: statically known invalid values diagnose during
typechecking/comptime, and runtime-dependent invalid values trap before a
`char` is constructed. Float, bool, and pointer sources are rejected. This
keeps String UTF-8 encoding valid without changing the representation of
arbitrary byte vectors.

Permanent Luna compiler fixtures cover invalid surrogate endpoints, values
above U+10FFFF, negative values and signed widening, u32/u128 maximum,
float/bool/pointer cast rejection, and runtime invalid values. Dynamic positive
controls cover all 12 integer types and char identity. Narrow u16 surrogate,
i8 negative, and u128 high-bit runtime controls check the width-specific
guards. A public CLI test rebuilds fresh
sysroot artifacts and checks valid and invalid behavior with both source-only
and artifact-only provider roots; it asserts that source fallback is absent.
On the current Windows worktree, the scalar matrix passes **1/1**, the
formatting source/artifact acceptance passes **2/2**, public CLI formatting
passes **1/1**, sysroot invariants pass **5/5** (32 providers / 91 edges), and
the Phase 5 whole-file I/O regression passes **3/3**. The expanded char CLI
matrix completed on 2026-10-01 with `CHAR_SCALAR_EXIT_CODE=0`: two positive
fixtures, ten compile-time rejection fixtures, and six runtime-abort fixtures
are verified in each provider mode. Runtime controls require an abort status,
not merely any unsuccessful execution.

The Unicode follow-up has fresh native Ubuntu/macOS evidence on commit
`765b9101be47ac91abb00bc596f5292e52e6e201`. Workflow run
[36800606504](https://github.com/17thedevv/luna/actions/runs/36800606504)
passed Ubuntu and macOS formatting acceptance and the complete Ubuntu
workspace regression. The full workspace job ran
`cargo test --workspace -- --test-threads=1` and completed successfully; its
post-job whitespace check also passed.

A Windows full-workspace attempt on the same commit exited 1 during
`stdlib_string_acceptance_tests::test_s14_string_source_and_llib_parity` with
`LLVM ERROR: IO failure on output stream: no space on device`. The system C:
drive had zero free bytes. Earlier Windows focused formatting, Unicode
source/artifact, and sysroot suites passed. This local full-run is recorded as
environment-limited, not as a product test failure or a successful Windows
workspace run; native Ubuntu is the completed full-workspace gate.

Compiler Change
    Capability: Unicode scalar validity at integer-to-char construction.
    Why stdlib exposed it: String's safe UTF-8 encoder accepted char values.
    Why it is generic: the checks apply to Luna's primitive semantic char type.
    User-defined type benefiting: any ordinary user API or aggregate containing char.
    Tests: standalone char_cast_*.ln fixtures and fresh source/artifact CLI parity.
    New intrinsic/lang_item?: NO.
    Stdlib-specific branch?: NO.

The earlier full command
`cargo test --workspace -- --test-threads=1` completed on Windows with exit
code 0 for the earlier implementation commit. Phase 6 CI run
[36694791949](https://github.com/17thedevv/luna/actions/runs/36694791949)
completed successfully on 2026-09-30: Ubuntu 24.04 formatting acceptance,
macOS 15 Intel formatting acceptance, and the full Ubuntu workspace regression
all concluded with success. The workflow targets commit
`dc123dfb0bd8748b0dcb259fc779b3d2f9da3c7c`.

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
Design authority decided on 2026-09-30 to track it outside Phase 6, so it does
not block formatting review. The formatter boundary acceptance uses the valid
arithmetic construction and does not treat the incorrect literal result as
expected behavior.

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

Phase 6 remains NOT FROZEN pending design-authority review. Windows focused
acceptance and native Ubuntu/macOS acceptance passed on the current commit;
the current full Ubuntu workspace gate passed. The Windows full-workspace
attempt on this commit was limited by a full system drive as recorded above.
Design-authority decisions are recorded above.

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
regression passes on Windows and the native CI acceptance matrix. The
independent negative-i64-literal issue remains outside this repair.

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
semantic error category. The Phase 6 implementation is complete and verified
on Windows, Ubuntu, and macOS; it is READY FOR DESIGN/FREEZE REVIEW — NOT
FROZEN.
