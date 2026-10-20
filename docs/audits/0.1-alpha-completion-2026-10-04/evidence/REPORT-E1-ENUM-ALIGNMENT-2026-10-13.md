# E1 continuation: terminal ownership and generic enum alignment

**Status: focused repair PASS; final E-wave qualification pending.**
The maintainer approved the Option current slot, then separately approved generic
enum alignment repair and compiler24 → 25. No E-wave closure is claimed here.

## Original failures and first divergence

The RangeInclusive MAX failure is preserved in the
[earlier report](REPORT-E1-RANGE-BLOCKER-2026-10-13.md). The approved Option slot
removes terminal stepping and passes all ten builtin integer boundaries.
Its non-Copy Counter control also passes. A stronger Probe containing i128
crashes with Windows access violation 0xC0000005, not a Luna arithmetic trap.

The [four-mode pre-repair acquisition](e1-enum-alignment-before-repair-2026-10-13.json)
records success for ordinary boundaries/Counter and failure for the wider Probe
and an independent safe Option reference-alignment check. The latter returns17:
the address of `&Wide` is not divisible by `alignof(Wide) = 16`. The failed
[focused log](e1-range-option-focused-before-alignment-2026-10-13.txt.gz) remains FAIL.
There is no iterator in the independent Option reproducer.

First incorrect representation: backend `map_type` used `{i32, [N x i64]}` for
every enum, with payload offset/alignment8. A payload containing i128 requires
alignment16 under the actual Windows GNU data layout. Generated payload stores,
loads and safe references therefore advertise alignment16 at an unaligned slot.
The VM separately omitted aggregate padding, returned alignment8 for i128 and
used fallback size8 for enums. Static enum storage had an independent copy of
the old representation algorithm.

## Semantic contract and Luna syntax

Every initialized variant must have enough correctly aligned payload storage;
safe references preserve validity, capability and receiver provenance. Enum
and aggregate `sizeof`/`alignof` agree with runtime storage in the tested target.
RangeInclusive yields the owned terminal item exactly once, without advancing
it or imposing Copy/Clone/Default. Ordinary Step arithmetic remains checked.

The implementation uses ordinary generic `Option<T>`, `match`, `rw` borrowing,
`life_from(self)`, `alignof(Self)` and `std::mem::replace`. No keyword, new
intrinsic, opcode, runtime entry point or compiler container-name branch is added.

## Generic compiler machinery

- Backend queries the configured LLVM TargetData for sizes and alignments.
  One enum representation helper is shared by runtime and constant lowering.
  It chooses the strongest payload alignment, uses a zero-length array to carry
  it without creating another owned value, and pads a word buffer large enough
  for every variant. The tag precedes the aligned payload slot.
- Dynamic construction/extraction and static packing use that same slot.
  Static initialization retains pointer relocations as LLVM constants.
- VM reflection calculates aggregate padding, concrete generic fields, arrays,
  enum payload storage and alignment for the currently admitted 64-bit GNU
  model. Unsupported/unresolved layouts and size overflow fail closed. This
  is not an arbitrary-target conformance claim.
- Option/Result as_ref/as_mut use `alignof(Self)` for their aligned payload offset.
  They still require the active variant, borrow live initialized storage and
  retain the receiver lifetime. A Result offset uses the whole enum alignment,
  including the other variant's alignment. No payload is moved or duplicated.
- RangeInclusive.current is Option<T>. A lexical shared borrow ends before
  mem::replace transfers the slot. Only a nonterminal item creates a successor.

The generic user Envelope provider exercises the same compiler machinery,
including multiple payload fields and nested enum arrays. There is no standard
library type-specific semantic exception. The compiler layout phase supplies
representation; TypeChecker/borrowck continue enforcing ordinary types and loans.

## Artifact boundary

Compiler25 rejects all prior compiler identities. Metadata9/MVIR5/format2 are
unchanged: semantic fields, variants and wire opcodes are unchanged; native enum
ABI changed. A [real compiler24 provider](compat24/provenance.json) was published
before the repair, with its actual source/object/LLVM IR and unmodified header.
Reader tests reject real22, real23 and real24 with component `compiler`.
Canonical Range layout changes also alter its public interface fingerprint;
a dependent produced against current:T is rejected after replacing the complete
dependency bundle with current:Option<T>, and succeeds after rebuilding.

## Focused verification and limits

The CLI range/alignment suite passes debug/source, debug/fresh-artifact,
release/source and release/fresh-artifact: 68 fixture acquisitions, plus native
old-layout baseline/rebuild and typed stale check/build rejection. Coverage:
all ten builtin Step types at MIN/MAX, singleton/inverted/exhausted/direct-slot
ranges, checked Step(MAX) traps, exact non-Copy drop/step counts, wider i128 Probe,
generic runtime and module constants, nested arrays and tuple payloads, static
pointer relocations, runtime/comptime layout parity and Option/Result projection.
The old current:T direct literal rejects E2001 and emits no executable.

The formerly failed driver range suite passes4. Backend enum layout invariant
and genuine old-artifact reader controls pass. Backend/MVIR unit suites,
module-constant regression and compiler25 CLI compatibility/relocation pass.
The first compatibility CLI rerun had an obsolete header24 assertion; it remains
FAIL, and a test-only correction to the version constant passes on rerun.
The [focused acquisition summary](e1-compiler25-focused-summary-2026-10-13.json)
links lossless logs and development source hashes. It also records the fail-closed
LLVM array-width check. Exact-pin workspace gates remain separate; this report
does not infer a full PASS from focused evidence.

Windows GNU native execution is tested. Linux GNU layout descriptor tests are
not native execution; Ubuntu GNU normal/ASan remains NOT VERIFIED for R5.
The VM's admitted 64-bit model does not establish cross-target reflection.
Borrowed-enum-pattern capability is not claimed; Range uses the existing
Option.as_ref API. Standalone comptime statement support remains the approved
E4005 gap. E2 stays CLOSED. Candidate remains at 6017ba8d until three consecutive
workspace passes and the remaining closure evidence are complete.

**SKILL IMPACT: none.** Existing capability/ownership/ABI guidance already requires
generic payload and negative/parity coverage; the concrete defect belongs here
and in permanent regressions. No new skill authority or language rule is invented.
