// =============================================================================
// luna/runtime/memory.h
//
// Luna Runtime Memory Allocation ABI (v1 entries retained, v2 fallible entries)
// =============================================================================

#pragma once

#include "abi.h"

#ifdef __cplusplus
extern "C" {
#endif

// --- Tier 1: Public Stable ABI Contract ---------------------------------------

// Allocate `size` bytes with alignment `align`.
// Returns non-null pointer. On OOM or invalid alignment, panics deterministically.
void* __luna_alloc(size_t size, size_t align);

// Deallocate memory previously allocated by __luna_alloc or __luna_realloc.
// If ptr is null or zero-size sentinel, behaves as no-op.
void  __luna_dealloc(void* ptr, size_t size, size_t align);

// Reallocate memory block preserving alignment.
void* __luna_realloc(void* ptr, size_t old_size, size_t old_align, size_t new_size);

// ABI v2. Output slot must be valid, writable and disjoint from the old block.
// Failure clears *out_ptr and leaves the old allocation/ownership unchanged.
// Raw pointer provenance and exact old allocation metadata remain preconditions.
#define LUNA_ALLOC_OK              ((int32_t)0)
#define LUNA_ALLOC_OUT_OF_MEMORY   ((int32_t)1)
#define LUNA_ALLOC_SIZE_OVERFLOW   ((int32_t)2)
#define LUNA_ALLOC_INVALID_ALIGN   ((int32_t)3)
int32_t __luna_try_alloc(size_t size, size_t align, void** out_ptr);
int32_t __luna_try_realloc(void* ptr, size_t old_size, size_t align,
                         size_t new_size, void** out_ptr);

// --- Tier 2: Internal Runtime Implementation Symbols (Private) ----------------

// Query if a pointer is the shared zero-size sentinel (internal tests only).
int   __luna_is_zero_sentinel(const void* ptr);

// Optional runtime memory primitives (fallback for embedded/freestanding).
void* __luna_mem_copy(void* dst, const void* src, size_t n);
void* __luna_mem_move(void* dst, const void* src, size_t n);
void* __luna_mem_set(void* dst, int val, size_t n);

#ifdef __cplusplus
} // extern "C"
#endif
