// =============================================================================
// luna/runtime/io.h
//
// Luna Runtime Primitive Stdio ABI (FROZEN)
// =============================================================================

#pragma once

#include "abi.h"

#ifdef __cplusplus
extern "C" {
#endif

void __luna_print(const uint8_t* str, size_t len);
void __luna_println(const uint8_t* str, size_t len);
void __luna_eprintln(const uint8_t* str, size_t len);

// --- Whole-File I/O (ABI v1) -------------------------------------------------

// Reads an entire file into a newly allocated buffer (allocated via __luna_alloc).
// Rejects paths with embedded NUL (LUNA_STATUS_INVALID_ARGUMENT).
// On success:
//   *out_ptr receives buffer
//   *out_len receives actual bytes read
//   *out_cap receives allocated capacity (*out_len <= *out_cap)
//   returns LUNA_STATUS_OK.
// Ownership is transferred to caller; free with __luna_buffer_free(*out_ptr, *out_cap).
int32_t __luna_read_file(
    const uint8_t*  path_ptr,
    size_t          path_len,
    uint8_t**       out_ptr,
    size_t*         out_len,
    size_t*         out_cap
);

// Writes data_len bytes to path. Creates or truncates the file.
// Rejects paths with embedded NUL (LUNA_STATUS_INVALID_ARGUMENT).
// Returns LUNA_STATUS_OK on success, or a negative LunaStatus error code.
int32_t __luna_write_file(
    const uint8_t*  path_ptr,
    size_t          path_len,
    const uint8_t*  data_ptr,
    size_t          data_len
);

// --- Stdin Line Input (ABI v1) -----------------------------------------------

// Reads a single line from standard input up to delimiter '\n' or EOF.
// Strips trailing '\n' and '\r' (CRLF normalization).
// Returns:
//   LUNA_STATUS_OK: line read (out_len is length; out_cap is buffer capacity).
//   LUNA_STATUS_EOF: EOF reached before any characters read (*out_ptr = NULL, *out_len = 0).
//   Negative LunaStatus on error.
int32_t __luna_read_line(
    uint8_t**       out_ptr,
    size_t*         out_len,
    size_t*         out_cap
);

// Release a runtime-allocated I/O buffer with known capacity
void __luna_buffer_free(uint8_t* ptr, size_t cap);

// --- Console Stdio Stream ABI ------------------------------------------------
int64_t __luna_stdin_read(uint8_t* buf, size_t len);
int64_t __luna_stdout_write(const uint8_t* buf, size_t len);
int32_t __luna_stdout_flush(void);
int64_t __luna_stderr_write(const uint8_t* buf, size_t len);
int32_t __luna_stderr_flush(void);

// --- File Stream ABI ---------------------------------------------------------
void*   __luna_file_open(
    const uint8_t*  path_ptr,
    size_t          path_len,
    int32_t         read_mode,
    int32_t         write_mode,
    int32_t         create_mode,
    int32_t         truncate_mode,
    int32_t         append_mode
);
int64_t __luna_file_read(void* handle, uint8_t* buf, size_t len);
int64_t __luna_file_write(void* handle, const uint8_t* buf, size_t len);
int64_t __luna_file_seek(void* handle, int64_t offset, int32_t whence);
int32_t __luna_file_flush(void* handle);
void    __luna_file_close(void* handle);

// --- File System / Directory ABI ---------------------------------------------
int32_t __luna_fs_create_dir(const uint8_t* path_ptr, size_t path_len);
int32_t __luna_fs_remove_file(const uint8_t* path_ptr, size_t path_len);
int32_t __luna_fs_remove_dir(const uint8_t* path_ptr, size_t path_len);
int32_t __luna_fs_metadata(
    const uint8_t*  path_ptr,
    size_t          path_len,
    uint64_t*       out_size,
    int32_t*        out_is_dir
);
void*   __luna_fs_opendir(const uint8_t* path_ptr, size_t path_len);
int32_t __luna_fs_readdir(void* handle, uint8_t* out_buf, size_t max_len, size_t* out_len, int32_t* out_is_dir);
void    __luna_fs_closedir(void* handle);
int32_t __luna_fs_canonicalize(const uint8_t* path_ptr, size_t path_len, uint8_t* out_buf, size_t max_len, size_t* out_len);

// --- Process Execution ABI ---------------------------------------------------
void*    __luna_process_spawn(const uint8_t* cmd_ptr, size_t cmd_len);
int32_t  __luna_process_wait(void* handle, int32_t* out_exit_code);
int32_t  __luna_process_kill(void* handle);
uint32_t __luna_process_id(void* handle);
void     __luna_process_close(void* handle);
int32_t  __luna_process_output(
    const uint8_t* cmd_ptr,
    size_t cmd_len,
    int32_t* out_exit_code,
    uint8_t** out_out_ptr,
    size_t* out_out_len,
    size_t* out_out_cap,
    uint8_t** out_err_ptr,
    size_t* out_err_len,
    size_t* out_err_cap
);

#ifdef __cplusplus
}
#endif
