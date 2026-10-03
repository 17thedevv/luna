// =============================================================================
// runtime/src/platform/posix/sync.c
//
// Luna Runtime — Sync ABI (POSIX Platform Implementation)
// =============================================================================

#include "luna/runtime/sync.h"
#include <pthread.h>
#include <stdlib.h>
#include <time.h>
#include <errno.h>

LunaMutexHandle __luna_mutex_create(void) {
    pthread_mutex_t* mutex = (pthread_mutex_t*)malloc(sizeof(pthread_mutex_t));
    if (!mutex) return NULL;
    if (pthread_mutex_init(mutex, NULL) != 0) {
        free(mutex);
        return NULL;
    }
    return (LunaMutexHandle)mutex;
}

void __luna_mutex_destroy(LunaMutexHandle handle) {
    if (!handle) return;
    pthread_mutex_destroy((pthread_mutex_t*)handle);
    free(handle);
}

void __luna_mutex_lock(LunaMutexHandle handle) {
    if (handle) {
        pthread_mutex_lock((pthread_mutex_t*)handle);
    }
}

int __luna_mutex_try_lock(LunaMutexHandle handle) {
    if (!handle) return 0;
    return pthread_mutex_trylock((pthread_mutex_t*)handle) == 0 ? 1 : 0;
}

void __luna_mutex_unlock(LunaMutexHandle handle) {
    if (handle) {
        pthread_mutex_unlock((pthread_mutex_t*)handle);
    }
}

// Condition Variable (POSIX pthreads)
LunaCondvarHandle __luna_condvar_create(void) {
    pthread_cond_t* cv = (pthread_cond_t*)malloc(sizeof(pthread_cond_t));
    if (!cv) return NULL;
    if (pthread_cond_init(cv, NULL) != 0) {
        free(cv);
        return NULL;
    }
    return (LunaCondvarHandle)cv;
}

void __luna_condvar_destroy(LunaCondvarHandle handle) {
    if (!handle) return;
    pthread_cond_destroy((pthread_cond_t*)handle);
    free(handle);
}

void __luna_condvar_wait(LunaCondvarHandle cv, LunaMutexHandle mx) {
    if (cv && mx) {
        pthread_cond_wait((pthread_cond_t*)cv, (pthread_mutex_t*)mx);
    }
}

int __luna_condvar_wait_timeout(LunaCondvarHandle cv, LunaMutexHandle mx, uint64_t nanoseconds) {
    if (!cv || !mx) return 0;
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    ts.tv_sec += nanoseconds / 1000000000ULL;
    ts.tv_nsec += nanoseconds % 1000000000ULL;
    if (ts.tv_nsec >= 1000000000L) {
        ts.tv_sec += 1;
        ts.tv_nsec -= 1000000000L;
    }
    int rc = pthread_cond_timedwait((pthread_cond_t*)cv, (pthread_mutex_t*)mx, &ts);
    return rc == 0 ? 1 : 0;
}

void __luna_condvar_signal(LunaCondvarHandle handle) {
    if (handle) {
        pthread_cond_signal((pthread_cond_t*)handle);
    }
}

void __luna_condvar_broadcast(LunaCondvarHandle handle) {
    if (handle) {
        pthread_cond_broadcast((pthread_cond_t*)handle);
    }
}

// Atomics (64-bit)
uint64_t __luna_atomic_load_u64(const volatile uint64_t* ptr) {
    return __atomic_load_n(ptr, __ATOMIC_SEQ_CST);
}

void __luna_atomic_store_u64(volatile uint64_t* ptr, uint64_t val) {
    __atomic_store_n(ptr, val, __ATOMIC_SEQ_CST);
}

uint64_t __luna_atomic_add_u64(volatile uint64_t* ptr, uint64_t val) {
    return __atomic_fetch_add(ptr, val, __ATOMIC_SEQ_CST);
}

uint64_t __luna_atomic_sub_u64(volatile uint64_t* ptr, uint64_t val) {
    return __atomic_fetch_sub(ptr, val, __ATOMIC_SEQ_CST);
}

uint64_t __luna_atomic_cas_u64(volatile uint64_t* ptr, uint64_t expected, uint64_t desired) {
    uint64_t exp = expected;
    __atomic_compare_exchange_n(ptr, &exp, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    return exp;
}

uint64_t __luna_atomic_swap_u64(volatile uint64_t* ptr, uint64_t val) {
    return __atomic_exchange_n(ptr, val, __ATOMIC_SEQ_CST);
}

// Atomics (32-bit)
uint32_t __luna_atomic_load_u32(const volatile uint32_t* ptr) {
    return __atomic_load_n(ptr, __ATOMIC_SEQ_CST);
}

void __luna_atomic_store_u32(volatile uint32_t* ptr, uint32_t val) {
    __atomic_store_n(ptr, val, __ATOMIC_SEQ_CST);
}

uint32_t __luna_atomic_add_u32(volatile uint32_t* ptr, uint32_t val) {
    return __atomic_fetch_add(ptr, val, __ATOMIC_SEQ_CST);
}

uint32_t __luna_atomic_sub_u32(volatile uint32_t* ptr, uint32_t val) {
    return __atomic_fetch_sub(ptr, val, __ATOMIC_SEQ_CST);
}

uint32_t __luna_atomic_cas_u32(volatile uint32_t* ptr, uint32_t expected, uint32_t desired) {
    uint32_t exp = expected;
    __atomic_compare_exchange_n(ptr, &exp, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    return exp;
}

uint32_t __luna_atomic_swap_u32(volatile uint32_t* ptr, uint32_t val) {
    return __atomic_exchange_n(ptr, val, __ATOMIC_SEQ_CST);
}
