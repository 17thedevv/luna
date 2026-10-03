// =============================================================================
// runtime/src/platform/windows/random.c
//
// Luna Runtime — Random Entropy ABI (Windows Platform Implementation)
// =============================================================================

#include "luna/runtime/random.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>

#pragma comment(lib, "bcrypt.lib")

int32_t __luna_random_bytes(void* buf, uint64_t len) {
    if (!buf || len == 0) return 0;

    NTSTATUS status = BCryptGenRandom(
        NULL,
        (PUCHAR)buf,
        (ULONG)len,
        BCRYPT_USE_SYSTEM_PREFERRED_RNG
    );

    if (BCRYPT_SUCCESS(status)) {
        return 0;
    }
    return (int32_t)status;
}

uint64_t __luna_random_entropy(void) {
    uint64_t entropy = 0;
    if (__luna_random_bytes(&entropy, sizeof(entropy)) == 0 && entropy != 0) {
        return entropy;
    }

    // High-resolution fallback entropy
    LARGE_INTEGER qpc;
    QueryPerformanceCounter(&qpc);

    FILETIME ft;
    GetSystemTimePreciseAsFileTime(&ft);
    uint64_t time_val = ((uint64_t)ft.dwHighDateTime << 32) | (uint64_t)ft.dwLowDateTime;

    uint64_t pid_tid = ((uint64_t)GetCurrentProcessId() << 32) | (uint64_t)GetCurrentThreadId();

    // 64-bit mixer (SplitMix64 style)
    uint64_t z = ((uint64_t)qpc.QuadPart ^ time_val ^ pid_tid) + 0x9E3779B97F4A7C15ULL;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}
