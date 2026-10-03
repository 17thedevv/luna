// =============================================================================
// runtime/src/platform/windows/sync.c
//
// Luna Runtime — Sync ABI (Windows Platform Implementation)
// =============================================================================

#include "luna/runtime/sync.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdlib.h>

LunaMutexHandle __luna_mutex_create(void) {
    CRITICAL_SECTION* cs = (CRITICAL_SECTION*)malloc(sizeof(CRITICAL_SECTION));
    if (!cs) return NULL;
    InitializeCriticalSection(cs);
    return (LunaMutexHandle)cs;
}

void __luna_mutex_destroy(LunaMutexHandle handle) {
    DeleteCriticalSection((CRITICAL_SECTION*)handle);
    free(handle);
}

void __luna_mutex_lock(LunaMutexHandle handle) {
    EnterCriticalSection((CRITICAL_SECTION*)handle);
}

int __luna_mutex_try_lock(LunaMutexHandle handle) {
    return TryEnterCriticalSection((CRITICAL_SECTION*)handle) ? 1 : 0;
}

void __luna_mutex_unlock(LunaMutexHandle handle) {
    LeaveCriticalSection((CRITICAL_SECTION*)handle);
}

// Condition Variable (Windows 8+)
LunaCondvarHandle __luna_condvar_create(void) {
    CONDITION_VARIABLE* cv = (CONDITION_VARIABLE*)malloc(sizeof(CONDITION_VARIABLE));
    if (!cv) return NULL;
    InitializeConditionVariable(cv);
    return (LunaCondvarHandle)cv;
}

void __luna_condvar_destroy(LunaCondvarHandle handle) {
    free(handle);
}

void __luna_condvar_wait(LunaCondvarHandle cv, LunaMutexHandle mx) {
    SleepConditionVariableCS((CONDITION_VARIABLE*)cv, (CRITICAL_SECTION*)mx, INFINITE);
}

int __luna_condvar_wait_timeout(LunaCondvarHandle cv, LunaMutexHandle mx, uint64_t nanoseconds) {
    DWORD ms = (DWORD)(nanoseconds / 1000000ULL);
    if (ms == 0 && nanoseconds > 0) ms = 1;
    BOOL ok = SleepConditionVariableCS((CONDITION_VARIABLE*)cv, (CRITICAL_SECTION*)mx, ms);
    return ok ? 1 : 0;
}

void __luna_condvar_signal(LunaCondvarHandle handle) {
    WakeConditionVariable((CONDITION_VARIABLE*)handle);
}

void __luna_condvar_broadcast(LunaCondvarHandle handle) {
    WakeAllConditionVariable((CONDITION_VARIABLE*)handle);
}

// Atomics (64-bit)
uint64_t __luna_atomic_load_u64(const volatile uint64_t* ptr) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_load_n(ptr, __ATOMIC_SEQ_CST);
#else
    return (uint64_t)InterlockedOr64((volatile LONG64*)ptr, 0);
#endif
}

void __luna_atomic_store_u64(volatile uint64_t* ptr, uint64_t val) {
#if defined(__GNUC__) || defined(__clang__)
    __atomic_store_n(ptr, val, __ATOMIC_SEQ_CST);
#else
    InterlockedExchange64((volatile LONG64*)ptr, (LONG64)val);
#endif
}

uint64_t __luna_atomic_add_u64(volatile uint64_t* ptr, uint64_t val) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_fetch_add(ptr, val, __ATOMIC_SEQ_CST);
#else
    return (uint64_t)InterlockedExchangeAdd64((volatile LONG64*)ptr, (LONG64)val);
#endif
}

uint64_t __luna_atomic_sub_u64(volatile uint64_t* ptr, uint64_t val) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_fetch_sub(ptr, val, __ATOMIC_SEQ_CST);
#else
    return (uint64_t)InterlockedExchangeAdd64((volatile LONG64*)ptr, -(LONG64)val);
#endif
}

uint64_t __luna_atomic_cas_u64(volatile uint64_t* ptr, uint64_t expected, uint64_t desired) {
#if defined(__GNUC__) || defined(__clang__)
    uint64_t exp = expected;
    __atomic_compare_exchange_n(ptr, &exp, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    return exp;
#else
    return (uint64_t)InterlockedCompareExchange64((volatile LONG64*)ptr, (LONG64)desired, (LONG64)expected);
#endif
}

uint64_t __luna_atomic_swap_u64(volatile uint64_t* ptr, uint64_t val) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_exchange_n(ptr, val, __ATOMIC_SEQ_CST);
#else
    return (uint64_t)InterlockedExchange64((volatile LONG64*)ptr, (LONG64)val);
#endif
}

// Atomics (32-bit)
uint32_t __luna_atomic_load_u32(const volatile uint32_t* ptr) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_load_n(ptr, __ATOMIC_SEQ_CST);
#else
    return (uint32_t)InterlockedOr((volatile LONG*)ptr, 0);
#endif
}

void __luna_atomic_store_u32(volatile uint32_t* ptr, uint32_t val) {
#if defined(__GNUC__) || defined(__clang__)
    __atomic_store_n(ptr, val, __ATOMIC_SEQ_CST);
#else
    InterlockedExchange((volatile LONG*)ptr, (LONG)val);
#endif
}

uint32_t __luna_atomic_add_u32(volatile uint32_t* ptr, uint32_t val) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_fetch_add(ptr, val, __ATOMIC_SEQ_CST);
#else
    return (uint32_t)InterlockedExchangeAdd((volatile LONG*)ptr, (LONG)val);
#endif
}

uint32_t __luna_atomic_sub_u32(volatile uint32_t* ptr, uint32_t val) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_fetch_sub(ptr, val, __ATOMIC_SEQ_CST);
#else
    return (uint32_t)InterlockedExchangeAdd((volatile LONG*)ptr, -(LONG)val);
#endif
}

uint32_t __luna_atomic_cas_u32(volatile uint32_t* ptr, uint32_t expected, uint32_t desired) {
#if defined(__GNUC__) || defined(__clang__)
    uint32_t exp = expected;
    __atomic_compare_exchange_n(ptr, &exp, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    return exp;
#else
    return (uint32_t)InterlockedCompareExchange((volatile LONG*)ptr, (LONG)desired, (LONG)expected);
#endif
}

uint32_t __luna_atomic_swap_u32(volatile uint32_t* ptr, uint32_t val) {
#if defined(__GNUC__) || defined(__clang__)
    return __atomic_exchange_n(ptr, val, __ATOMIC_SEQ_CST);
#else
    return (uint32_t)InterlockedExchange((volatile LONG*)ptr, (LONG)val);
#endif
}


