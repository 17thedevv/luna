// =============================================================================
// luna/runtime/time.h
//
// Luna Runtime Time ABI — Low-level OS/clock bindings only.
// =============================================================================

#pragma once

#include "abi.h"

#ifdef __cplusplus
extern "C" {
#endif

// Returns monotonic clock time in nanoseconds since an arbitrary point.
uint64_t __luna_time_monotonic_nanos(void);

// Returns system (wall clock) time since Unix Epoch (1970-01-01 00:00:00 UTC).
// out_secs receives seconds, out_nanos receives subsecond nanoseconds (0..999,999,999).
void __luna_time_system_epoch(uint64_t* out_secs, uint32_t* out_nanos);

#ifdef __cplusplus
} // extern "C"
#endif
