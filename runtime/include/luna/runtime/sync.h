// =============================================================================
// luna/runtime/sync.h
//
// Luna Runtime Synchronization ABI — Native mutex/condvar OS bindings.
// =============================================================================

#pragma once

#include "abi.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef void* LunaMutexHandle;
typedef void* LunaCondvarHandle;

// Mutex
LunaMutexHandle __luna_mutex_create(void);
void            __luna_mutex_destroy(LunaMutexHandle handle);
void            __luna_mutex_lock(LunaMutexHandle handle);
int             __luna_mutex_try_lock(LunaMutexHandle handle); // 1=acquired
void            __luna_mutex_unlock(LunaMutexHandle handle);

// Condition Variable
LunaCondvarHandle __luna_condvar_create(void);
void              __luna_condvar_destroy(LunaCondvarHandle handle);
void              __luna_condvar_wait(LunaCondvarHandle cv,
                                      LunaMutexHandle mx);
int               __luna_condvar_wait_timeout(LunaCondvarHandle cv,
                                              LunaMutexHandle mx,
                                              uint64_t nanoseconds);
void              __luna_condvar_signal(LunaCondvarHandle handle);
void              __luna_condvar_broadcast(LunaCondvarHandle handle);

// Atomics (64-bit)
uint64_t __luna_atomic_load_u64(const volatile uint64_t* ptr);
void     __luna_atomic_store_u64(volatile uint64_t* ptr, uint64_t val);
uint64_t __luna_atomic_add_u64(volatile uint64_t* ptr, uint64_t val);
uint64_t __luna_atomic_sub_u64(volatile uint64_t* ptr, uint64_t val);
uint64_t __luna_atomic_cas_u64(volatile uint64_t* ptr, uint64_t expected, uint64_t desired);
uint64_t __luna_atomic_swap_u64(volatile uint64_t* ptr, uint64_t val);

// Atomics (32-bit)
uint32_t __luna_atomic_load_u32(const volatile uint32_t* ptr);
void     __luna_atomic_store_u32(volatile uint32_t* ptr, uint32_t val);
uint32_t __luna_atomic_add_u32(volatile uint32_t* ptr, uint32_t val);
uint32_t __luna_atomic_sub_u32(volatile uint32_t* ptr, uint32_t val);
uint32_t __luna_atomic_cas_u32(volatile uint32_t* ptr, uint32_t expected, uint32_t desired);
uint32_t __luna_atomic_swap_u32(volatile uint32_t* ptr, uint32_t val);

#ifdef __cplusplus
} // extern "C"
#endif
