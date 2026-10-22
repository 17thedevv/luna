<!-- luna-doc-role: evidence -->

# Stdlib fallible allocation implementation

Started 2026-10-21, Asia/Bangkok, after the maintainer selected STD-D06 and
explicitly requested implementation. **IMPLEMENTATION / VALIDATION IN PROGRESS**.
This is the first vertical slice of the whole stdlib redesign, not stable
completion. The user's installed alpha is unchanged.

The [policy/design](../../proposals/stdlib-redesign-0.1/allocator.md) and
[runtime ABI 2 contract](../../runtime/abi-v2-allocation.md) describe the scope.
The immutable earlier parser inventory still describes 49 providers at 264390da.
This implementation adds the explicit alloc_error provider, making 50 manifest
providers; the count is not evidence that all their APIs are correct.

[API delta](api-delta.json) maps the five existing factory declaration IDs to
their preferred type-associated constructors, lists the six added fallible
operations and identifies the changed overflow behavior. It is only this slice;
the complete pre-stable API migration map remains STABLE-016.

## Implementation

- Runtime: status/out-pointer try_alloc and try_realloc; failure preserves the
  old block. Legacy aborting entry points retain their typed panic behavior.
- Stdlib: std::AllocError and std::AllocFailure<T>; Vec::try_with_capacity,
  try_reserve and try_push; Box::try_new; String::try_with_capacity/try_reserve.
- Constructors: Vec/Box/String get type-associated constructors. Existing
  factory names temporarily remain migration bridges; old u64 size/index APIs
  are not silently declared fully migrated to STD-D05.
- Overflow is checked before allocation, including alignment round-up and
  growth-policy bounds. Previously vec_with_capacity returned an empty vector
  on element-count overflow; that path now aborts instead of reporting success.
- Deterministic OOM injection is compiled only into a separate test archive;
  production symbols were checked to exclude all __luna_test_* hooks.

## Compiler repairs discovered by this work

| Record | Generic defect | Repair |
| --- | --- | --- |
| [C-GAP-13](compiler-gap-13.md) | Qualified owner/impl binders and their monomorphization | b40bf923; user-defined direct/indirect, specialized self and bound controls |
| [C-GAP-14](compiler-gap-14.md) | Imported extern identity lost before FFI borrow analysis | Remap source facts and preserve existing canonical ExternFunction kind |
| [C-GAP-15](compiler-gap-15.md) | Omitted owner inference and callable values bypassing impl bounds | afa2b825; independent impl/method variables and bound obligations after inference |

These repairs introduce no container branch, new intrinsic, lang item or
language semantic rule. Native source/artifact CLI regression covers each.

## Verification state

Windows [focused evidence at 5e1c7356](evidence/windows-5e1c7356/receipt.json)
passes all five CLI suites (26 Rust tests), 209 semantic tests and six sysroot
invariant tests. Constructor, allocator, imported FFI and I/O controls execute
against isolated source-only and freshly built artifact-only roots. Added
controls cover borrowed inputs carried by allocation failures, mutation after
those owners die, ordinary overflow aborts and constructor inference. The
Windows runtime test archive fallback was exercised without a prebuilt override.

Both C suites also pass a clean GNU
[Windows Debug build with assertions active](evidence/windows-runtime-asserts-1548f1b6/receipt.json).
The runtime tree is unchanged from the allocator implementation pin.

The [older workspace attempts](evidence/windows-workspace-attempts/receipt.json)
are incomplete/superseded, not workspace PASS. Their original raw summaries and
streams are archived with hashes, including setup failures and interruptions.
The subsequent workspace run at 5e1c7356 was cancelled before completion in
[Ubuntu run 39634996889](https://github.com/17thedevv/luna/actions/runs/39634996889).
Its runtime and four focused CLI suites passed; workspace is incomplete and
ASan was skipped. The [hashed raw artifact receipt](evidence/ubuntu-5e1c7356/receipt.json)
preserves this result without promoting it to workspace PASS.

Ubuntu [run 39567793256](https://github.com/17thedevv/luna/actions/runs/39567793256)
passes runtime ABI/deterministic failure (2 C suites), all four public CLI suites
including source/artifact controls, and both ASan C-runtime suites. The
[receipt](evidence/ubuntu-b9878c70/receipt.json) retains the four raw artifact
files and their SHA-256 values. Downloaded ZIP SHA-256 matches GitHub's artifact
digest: 1018c40c54db0fd7b1f832f0628d37ef1dea81404611b661ff5af6da549c71e4.
No result here transfers to the final stable eight-lane or clean package gates.

Native callable success does not establish comptime callable support. The
[comptime probe](evidence/comptime-callable-gap/receipt.json) rejects E4005 with
both old and repaired compilers; artifact-mode comptime parity remains unknown.

## Remaining release work

Complete broad regression, reacquire Ubuntu/ASan on the repaired pin and validate
runtime/stdlib package compatibility. The existing alpha package scripts pin 49
providers and alpha metadata; they are not an ABI 2 stable distribution builder.
A new bundle must keep compiler, 50 provider/object pairs and runtime ABI 2
coherent. Migrate remaining size/index APIs, dependencies and API
catalog in subsequent STD-D05/STD-D01 slices; then rebuild and qualify packages
from the final stable candidate. No installation, tag, merge or release occurs
merely because this focused implementation compiles.

SKILL IMPACT: none. Existing semantic, provider/namespace, compiler boundary and
CLI-parity rules cover both generic defects. No new skill authority was created.
