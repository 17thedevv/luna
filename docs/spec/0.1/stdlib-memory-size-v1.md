<!-- luna-doc-role: adopted-contract -->

# STD-D05 — target-sized memory counts

Approved direction: maintainer chose usize for memory/container length, capacity
and index on 2026-10-21, then requested expansion under those API standards on
2026-10-22. This amendment applies that instruction to the memory API contract;
implementation and release qualification remain separate.

Unsigned counts that address in-memory storage SHALL use `usize`: allocation
size/alignment, raw slice lengths, byte/element copy counts, container lengths,
capacities and indices. Count units remain explicit: `std::mem` copies bytes;
`std::ptr::copy<T>` copies a count of T-sized elements.

The existing `slice_from_raw_parts` and `slice_from_raw_parts_mut` lang contracts
therefore have the canonical signatures, regardless of declaration spelling:

```luna
#[lang("slice_from_raw_parts")]
unsafe intrinsic fn view<T>(p: *T, count: usize) -> &[T];
#[lang("slice_from_raw_parts_mut")]
unsafe intrinsic fn view_mut<T>(p: *rw T, count: usize) -> &rw [T];
```

These remain trusted compiler bindings, not permissions for arbitrary providers
to register lang items. Existing unsafe admission, raw provenance, initialized
storage, alignment, aliasing, lifetime and owner-anchor requirements remain in
force. No new intrinsic or ownership primitive is added. Slice length remains
target-sized through semantic checking, portable metadata, MVIR and native code.

`usize` and same-width `u64` are distinct Luna types. A 64-bit representation
does not make an obsolete u64 intrinsic declaration canonical. Compiler protocol
26 rejects earlier artifacts; coherent providers and dependent bundles must be
rebuilt. Runtime ABI 2 and the other artifact version components are unchanged.

Fixed-width domain values, hash outputs, file offsets and cumulative stream
totals are outside this unsigned memory-count migration. Signed pointer offsets
and distances retain their current API pending their separate isize review.
This amendment does not certify 32-bit native support.
