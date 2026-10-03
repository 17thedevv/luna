// =============================================================================
// runtime/src/platform/windows/time.c
//
// Luna Runtime — Time ABI (Windows Platform Implementation)
// =============================================================================

#include "luna/runtime/time.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

static LARGE_INTEGER g_qpc_frequency;
static int g_qpc_initialized = 0;

static void init_qpc(void) {
    if (!g_qpc_initialized) {
        QueryPerformanceFrequency(&g_qpc_frequency);
        g_qpc_initialized = 1;
    }
}

uint64_t __luna_time_monotonic_nanos(void) {
    init_qpc();
    LARGE_INTEGER counter;
    QueryPerformanceCounter(&counter);

    uint64_t freq = (uint64_t)g_qpc_frequency.QuadPart;
    if (freq == 0) return 0;

    uint64_t count = (uint64_t)counter.QuadPart;
    uint64_t whole = count / freq;
    uint64_t rem = count % freq;
    return whole * 1000000000ULL + (rem * 1000000000ULL) / freq;
}

void __luna_time_system_epoch(uint64_t* out_secs, uint32_t* out_nanos) {
    FILETIME ft;
    GetSystemTimePreciseAsFileTime(&ft);

    ULARGE_INTEGER ul;
    ul.LowPart = ft.dwLowDateTime;
    ul.HighPart = ft.dwHighDateTime;

    // 100-nanosecond intervals between 1601-01-01 and 1970-01-01:
    // 11644473600 seconds * 10,000,000 intervals/second = 116444736000000000ULL
    const uint64_t UNIX_EPOCH_DIFF = 116444736000000000ULL;

    if (ul.QuadPart >= UNIX_EPOCH_DIFF) {
        uint64_t intervals = ul.QuadPart - UNIX_EPOCH_DIFF;
        if (out_secs) {
            *out_secs = intervals / 10000000ULL;
        }
        if (out_nanos) {
            *out_nanos = (uint32_t)((intervals % 10000000ULL) * 100ULL);
        }
    } else {
        if (out_secs) *out_secs = 0;
        if (out_nanos) *out_nanos = 0;
    }
}
