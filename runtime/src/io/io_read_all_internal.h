// =============================================================================
// Internal whole-file read loop shared by the runtime and ABI tests.
// This is not part of the public runtime ABI.
// =============================================================================

#pragma once

#include "luna/runtime/io.h"
#include "luna/runtime/memory.h"

#include <stdint.h>
#include <stddef.h>

typedef enum LunaRtReadResult {
    LUNA_RT_READ_DATA,
    LUNA_RT_READ_EOF,
    LUNA_RT_READ_INTERRUPTED,
    LUNA_RT_READ_ERROR
} LunaRtReadResult;

typedef LunaRtReadResult (*LunaRtReadNext)(
    void* context,
    uint8_t* destination,
    size_t capacity,
    size_t* out_bytes
);

static inline int32_t luna_rt_read_all(
    LunaRtReadNext read_next,
    void* context,
    size_t initial_capacity_hint,
    uint8_t** out_ptr,
    size_t* out_len,
    size_t* out_cap
) {
    if (!read_next || !out_ptr || !out_len || !out_cap) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

    *out_ptr = NULL;
    *out_len = 0;
    *out_cap = 0;

    size_t capacity = initial_capacity_hint;
    if (capacity == 0) {
        capacity = 4096;
    }

    uint8_t* buffer = (uint8_t*)__luna_alloc(capacity, 1);
    size_t total_read = 0;

    for (;;) {
        if (total_read == capacity) {
            uint8_t extra_byte = 0;
            size_t bytes_read = 0;
            LunaRtReadResult result = read_next(
                context,
                &extra_byte,
                1,
                &bytes_read
            );
            if (result == LUNA_RT_READ_INTERRUPTED) {
                continue;
            }
            if (result == LUNA_RT_READ_EOF) {
                break;
            }
            if (result != LUNA_RT_READ_DATA || bytes_read != 1) {
                __luna_dealloc(buffer, capacity, 1);
                return LUNA_STATUS_IO_ERROR;
            }
            if (capacity > SIZE_MAX / 2) {
                __luna_dealloc(buffer, capacity, 1);
                return LUNA_STATUS_IO_ERROR;
            }
            size_t new_capacity = capacity * 2;
            buffer = (uint8_t*)__luna_realloc(buffer, capacity, 1, new_capacity);
            buffer[total_read] = extra_byte;
            total_read++;
            capacity = new_capacity;
            continue;
        }

        size_t available = capacity - total_read;
        size_t bytes_read = 0;
        LunaRtReadResult result = read_next(
            context,
            buffer + total_read,
            available,
            &bytes_read
        );

        if (result == LUNA_RT_READ_INTERRUPTED) {
            continue;
        }
        if (result == LUNA_RT_READ_EOF) {
            break;
        }
        if (result != LUNA_RT_READ_DATA || bytes_read == 0 || bytes_read > available) {
            __luna_dealloc(buffer, capacity, 1);
            return LUNA_STATUS_IO_ERROR;
        }
        total_read += bytes_read;
    }

    if (total_read == 0) {
        __luna_dealloc(buffer, capacity, 1);
        buffer = (uint8_t*)__luna_alloc(0, 1);
        capacity = 0;
    }

    *out_ptr = buffer;
    *out_len = total_read;
    *out_cap = capacity;
    return LUNA_STATUS_OK;
}
