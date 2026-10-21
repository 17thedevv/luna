<!-- luna-doc-role: guidance -->

# Runtime ABI 2: fallible allocation extension

Implementation contract for STD-D06, 2026-10-21. The maintainer authorized
implementation after selecting two-tier allocation. This extension is under
validation on `stdlib-redesign`; it is not shipped or a stable qualification.
The language version and LLIB protocols are independent of this revision.

ABI 2 retains the [v1 core entry points and contracts](abi-v1.md).
`__luna_runtime_abi_version()` reports 2. New declarations live in
`runtime/include/luna/runtime/memory.h`:

```c
int32_t __luna_try_alloc(size_t size, size_t align, void **out_ptr);
int32_t __luna_try_realloc(void *ptr, size_t old_size, size_t align,
                         size_t new_size, void **out_ptr);
```

| Allocation status | Value | Output |
| --- | --- | --- |
| LUNA_ALLOC_OK | 0 | Non-null pointer, including the zero-size sentinel |
| LUNA_ALLOC_OUT_OF_MEMORY | 1 | NULL |
| LUNA_ALLOC_SIZE_OVERFLOW | 2 | NULL |
| LUNA_ALLOC_INVALID_ALIGN | 3 | NULL |

These allocation status values are a separate domain from LunaStatus I/O codes.
The output slot must be valid, writable, pointer-aligned and disjoint from the
old block. It is cleared before request validation. No error reporting allocates
memory or calls user code. Raw allocation provenance, exact metadata and unique
ownership remain caller preconditions; an invalid raw pointer is not a
recoverable layout error.

Alignment is a power of two in 1..4096. Validate alignment before zero-size
handling. A zero-size success returns the runtime-owned sentinel aligned 4096;
it provides no writable bytes and deallocation is a no-op. Size round-up must
not wrap; callers separately check element-count multiplication/addition.
Luna usize corresponds to target-native size_t. Advertised native target scope
for this implementation remains the existing two 64-bit GNU targets.

Reallocation from NULL/sentinel (old_size 0) acts as allocation. With valid old
metadata, a new size of zero deallocates and succeeds with the sentinel. Other
successes preserve min(old_size, new_size) bytes, free the old block exactly
once and transfer ownership of the returned block. On any reported failure the
old block/content/ownership remains unchanged. Keep the original pointer until
success; passing its only storage as out_ptr would overwrite that storage with
NULL on failure. Success invalidates all uses of the old allocation pointer.

`__luna_alloc` and `__luna_realloc` still never return NULL: OOM and round-up
overflow panic with L001. Invalid alignment retains L005. Panic remains
terminal; no unwinding/recovery mechanism was introduced. Runtime code knows
no container, Result, UTF-8 or Drop policy.

The separate `luna-runtime-alloc-test` archive compiles the same allocator with
deterministic failure budgets/counters. It is not installed, and production
runtime exports must not contain `__luna_test_*`. Its counters are single-process,
single-thread test instrumentation, not production allocation guarantees.

Public stdlib API, ownership on insertion failure and acceptance are recorded
in the [allocator design/implementation ledger](../proposals/stdlib-redesign-0.1/allocator.md).
Packaging must pair the new stdlib with ABI 2 and validate the runtime/stdlib
combination before release. An alpha ABI 1 runtime cannot supply the new symbols.
