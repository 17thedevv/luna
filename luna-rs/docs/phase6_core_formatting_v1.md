<!-- luna-doc-role: adopted-contract -->

> **Luna 0.1 — adopted-contract.** Retained detailed contract. Prior acceptance and freeze claims remain dated evidence; current release conformance is tracked separately. See the [versioned specification](../../docs/spec/0.1/README.md).

> **Integration boundary — 2026-10-03.** The prior maintainer freeze applies to 0394776 only. Later worktree additions are now being integrated; their merged implementation is not covered by that dated acceptance. Luna 0.1 release conformance remains blocked/unverified.

# Stage 7 — Phase 6: Core Formatting Foundation v1

Status: RESOLVED & FROZEN

Freeze date: 2026-10-01.
Freeze authority: human maintainer, by explicit approval in the task conversation:
"Phê duyệt freeze Phase 6 trên baseline 0394776".
Verified implementation: `039477650b8cd0b39d058c153c96e3253f02fc75`.

This freeze applies to Core Formatting Foundation v1 and its required generic
compiler repairs at that exact implementation baseline. It does not freeze the
entire current working tree, parallel Antigravity additions, a new phase, or the
two independent numeric defects documented below. It does not authorize a
main-branch merge. The normative contracts and explicit non-goals below are
frozen; particular formatting algorithms and test-harness mechanics are evidence
and implementation strategy, not additional public APIs.

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

Design authority approved the operator contract on 2026-10-01: arithmetic,
bitwise, shift, unary negation and compound assignment on `char` are rejected.
Comparisons, ordinary assignment and explicit integer casts remain available.
The restriction also applies after generic instantiation; it is not an encoder
workaround. Integer arithmetic followed by a checked integer-to-char cast is
the explicit way to construct another scalar.

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

## 2026-10-01 final operator and CLI closure — current record

A safe program could previously add U+D7FF and U+0001 as `char`, producing
U+D800. Both source and artifact modes emitted the invalid UTF-8 bytes
`ED A0 80`. Cast validation alone did not cover primitive operator results.

`luna-semantic/src/operators.rs` now owns the shared primitive restriction.
Type checking applies it to concrete operands; `MonoCollector` reapplies it
to substituted generic operands before MVIR/backend generation. This includes
generic bodies loaded from a fresh user-provider artifact, aliases and user
aggregate fields. No formatter, String, provider or method-name branch was
added; no runtime, backend representation or artifact metadata change was
needed for this repair.

Compiler Change
    Capability: preserve Unicode scalar validity across primitive operators.
    Why stdlib exposed it: valid char inputs could produce invalid String UTF-8.
    Why it is generic: one primitive semantic contract, including substituted generic bodies.
    User-defined type benefiting: UserValue { scalar: char }, user_math generic provider.
    Tests: char_operator_cli_parity, standalone char_operator/assignment/provider fixtures.
    New intrinsic/lang_item?: NO.
    Stdlib-specific branch?: NO.

Formatting acceptance now belongs entirely to the public CLI harness, not
internal driver helpers. `support/stdlib.rs` copies only current `.ln` sources
and the manifest into a clean sysroot, invokes `luna build-sysroot`, and then
constructs source-only and `.llib/.obj`-only roots. It asserts that the artifact
root contains no provider source and the source root contains no artifacts.
Each compile uses an explicit sysroot. The harness owns and cleans only its
unique temporary directory. User-provider artifacts are built against the
fresh canonical dependency artifacts whose interface fingerprints they record.

The matrix contains eight formatting executables and four compile-negative
fixtures in each mode. Negative programs run through both CLI `check` and
CLI `build`; semantic messages are compared, excluding presentation-only
source snippets and normalizing session-local type IDs. Positive output must
decode as **strict** UTF-8, match the exact text oracle (only platform newline
normalization), and have identical raw stdout, stderr and exit status between
provider modes.

The new failing-writer matrix exercises all 12 integers, both bools, a Unicode
char, nonempty Unicode String, empty String and all four FileError variants.
It tests every character boundary, including exact success: 21 values and 285
sink scenarios per mode. Each case checks accepted prefix checksum/count,
first-failure propagation and absence of subsequent writes.

The char-operator matrix contains 28 direct/generic consumer negatives and
four negatives instantiated from an ordinary user-provider artifact. Every
case must fail before codegen with `E_INVALID_CHAR_OPERATOR` in both modes and
both CLI commands. Positive controls execute comparisons, assignment,
integer arithmetic/casts, and generic integer/float operations. Existing
Unicode cast checks remain: two positive, ten compile-negative and six
runtime-abort fixtures in each mode.

### Exact-commit verification boundary

The verified frozen implementation is
`039477650b8cd0b39d058c153c96e3253f02fc75`. It includes the char-operator repair
and public CLI parity matrix (`cacc59f`), followed by a test-harness portability
repair: executable outputs live in a dedicated directory, not at a POSIX path
already occupied by the source-only sysroot. The initial native run on cacc59f
failed on that directory/output-name collision; it was not accepted as evidence.

Fresh native run
[36814704447](https://github.com/17thedevv/luna/actions/runs/36814704447)
targets exactly 0394776. Ubuntu 24.04 and macOS 15 Intel both pass formatting
CLI 2/2, char-scalar CLI 1/1, char-operator CLI 2/2, trait-bound 1/1 and integer
cast 1/1. The separate Ubuntu full-workspace job completes
`cargo test --workspace -- --test-threads=1` with exit 0: 1,222 tests passed,
zero failed and one pre-existing ignored test
(`mutable_references::sem_mutref_06_invalid_fixture`). All three native jobs
also pass `git diff --check`. No macOS full-workspace result is claimed.

Windows verification uses an independent checkout pinned to 0394776, an
independent Cargo target directory and fresh canonical sysroot artifacts.
Concurrent, uncommitted stdlib/runtime additions in the main working tree are
excluded. On this checkout the focused formatting 2/2, char-scalar 1/1,
char-operator 2/2, operator unit 1/1, generic trait-bound 1/1, integer cast 1/1,
enum tag/codegen 1/1, sysroot invariants 5/5 and Phase 5 whole-file I/O 3/3
all pass with exit 0. The independent Windows full command
`cargo test --workspace -- --test-threads=1` completed on 2026-10-01 with
**exit 0**: 1,222 passed, zero failed and the same one pre-existing ignored
test as Ubuntu. All 163 unit/integration/doc-test binary results were captured;
none reports a failure. `git diff --check` on the isolated checkout also exits 0.
The verification log is `D:/luna-phase6-verification-20261001/isolated-workspace.log`.

The previous Windows full run on the concurrently changing main worktree
exited 101 with five String acceptance failures reporting strict dependency
fingerprint mismatches. It is not a passing gate and does not establish a
Phase 6 defect on the isolated commit; the independent full run replaces that
contaminated verification attempt. In particular, all 16 String acceptance
tests pass on the isolated commit, including all five previously failing cases.

Earlier CI runs below are historical, not evidence for the current changes.
Runtime/ABI source is unchanged in this Phase 6 implementation, so no ABI rerun
is required. The isolated baseline remains 32 providers / 91 direct edges; it
does not certify later providers contributed in parallel. The main working
tree's unrelated whitespace/formatting debt is not declared clean; the Phase 6
documentation patch passes its own scoped `git diff --check`.

All Phase 6 closure gates are complete for this implementation commit. No
additional compiler, runtime or stdlib behavior was changed during the final
isolated verification. After reviewing this evidence, the human maintainer
explicitly approved the Phase 6 freeze on 2026-10-01. No main-branch merge is
included. Review of the concurrently added stdlib capabilities is a separate
subsequent task. Existing compiler warnings, the one pre-existing ignored test
and repository-wide formatting debt are recorded limitations, not claims of a
warning-free or globally clean repository.

An independent pre-existing defect was discovered by an integer control:
MVIR assignment lowering ignores `AssignOp`, so `value += 34` acts as ordinary
assignment. `numeric_compound_assignment_diagnostic.ln` expects the correct
sum and currently exits 1; it is diagnostic evidence, not a passing regression.
Formatting v1 does not use compound assignment. This defect is recorded
outside the formatting implementation and is not repaired or endorsed here.
The separately approved signed-minimum literal issue also remains outside
Phase 6. The maintainer's freeze approval explicitly retains both defects
outside Phase 6; neither incorrect behavior is approved or frozen. No gap ID is
invented for either finding.

## Historical implementation evidence — through 765b910

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

Historical Windows focused evidence:

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

The `fmt` provider and its initial `String` sink changed the Phase 6 sysroot
baseline to 32 providers and 90 direct dependency edges. At the frozen
implementation baseline, `file` depends on `fmt` for `FileError` formatting,
bringing the baseline to 32 providers / 91 direct edges. Both values are
asserted by the sysroot DAG invariant test at 0394776, not by an inventory of
the concurrently expanded working tree.

Phase 5 has native Linux/macOS build/runtime evidence. Its generic enum-pattern
lowering blocker was fixed and merged to `main`; see the
[re-audit and resolution record](phase5_reaudit_2026_09_30.md). Phase 5 remains
NOT FROZEN pending design-authority review.

Phase 6 is RESOLVED & FROZEN by maintainer approval on 2026-10-01, for the
0394776 implementation baseline only. Historical results in this record apply
to their explicitly named commits and do not replace the final exact-commit
verification in the 2026-10-01 section.

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
on Windows, Ubuntu, and macOS at the scopes recorded above, and is RESOLVED &
FROZEN by the maintainer's explicit approval. Subsequent changes must preserve
this contract and rerun the affected CLI/fresh-artifact regression gates.
