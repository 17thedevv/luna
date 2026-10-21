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

Neither repair introduces a container branch, new intrinsic, lang item or
language semantic rule. Source/artifact CLI regression is required for both.

## Verification state

Initial Windows runtime ABI and deterministic failure suites pass. Focused
constructor and allocator matrices have passed native source/artifact runs;
final tightened negative controls and broader regression are being acquired.
Logs from failed intermediate probes are retained outside the repository and
are not counted as PASS. Final evidence will record command, input pin/tree,
exit and raw log digests, including any remaining failures.

The dedicated Ubuntu workflow runs runtime ABI, native public CLI matrices and
ASan C-runtime checks. Until its result is acquired, Ubuntu remains unverified.
No result here transfers to the final stable eight-lane or clean package gates.

## Remaining release work

Complete broad regression, Ubuntu/ASan acquisition and runtime/stdlib package
compatibility checks. Migrate remaining size/index APIs, dependencies and API
catalog in subsequent STD-D05/STD-D01 slices; then rebuild and qualify packages
from the final stable candidate. No installation, tag, merge or release occurs
merely because this focused implementation compiles.

SKILL IMPACT: none. Existing semantic, provider/namespace, compiler boundary and
CLI-parity rules cover both generic defects. No new skill authority was created.
