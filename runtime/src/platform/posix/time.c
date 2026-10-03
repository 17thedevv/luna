// =============================================================================
// runtime/src/platform/posix/time.c
//
// Luna Runtime — Time ABI (POSIX Platform Implementation)
// =============================================================================

#include "luna/runtime/time.h"
#include <time.h>

uint64_t __luna_time_monotonic_nanos(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

void __luna_time_system_epoch(uint64_t* out_secs, uint32_t* out_nanos) {
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    if (out_secs) {
        *out_secs = (uint64_t)ts.tv_sec;
    }
    if (out_nanos) {
        *out_nanos = (uint32_t)ts.tv_nsec;
    }
}
