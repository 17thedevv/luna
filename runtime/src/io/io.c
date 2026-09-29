// =============================================================================
// runtime/src/io/io.c
//
// Luna Runtime — Primitive Stdio, Whole-File I/O & Stdin Stream (ABI v1)
// =============================================================================

#include "luna/runtime/io.h"
#include "luna/runtime/memory.h"
#include "io_read_all_internal.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <fcntl.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <errno.h>
#endif

// --- Stdio Byte Stream Output ------------------------------------------------

void __luna_print(const uint8_t* str, size_t len) {
    if (str && len) {
        fwrite(str, 1, len, stdout);
        fflush(stdout);
    }
}

void __luna_println(const uint8_t* str, size_t len) {
    if (str && len) {
        fwrite(str, 1, len, stdout);
    }
    fputc('\n', stdout);
    fflush(stdout);
}

void __luna_eprintln(const uint8_t* str, size_t len) {
    if (str && len) {
        fwrite(str, 1, len, stderr);
    }
    fputc('\n', stderr);
    fflush(stderr);
}

// --- Runtime Buffer Free Helper ----------------------------------------------

void __luna_buffer_free(uint8_t* ptr, size_t cap) {
    __luna_dealloc((void*)ptr, cap, 1);
}

// --- Platform Path Helpers (Internal) -----------------------------------------

#if defined(_WIN32)
static wchar_t* luna_rt_path_to_wchar(const uint8_t* path_ptr, size_t path_len, wchar_t* stack_buf, size_t stack_cap) {
    int req_len = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, (const char*)path_ptr, (int)path_len, NULL, 0);
    if (req_len <= 0) {
        return NULL;
    }
    wchar_t* wbuf = stack_buf;
    if ((size_t)req_len + 1 > stack_cap) {
        wbuf = (wchar_t*)malloc(((size_t)req_len + 1) * sizeof(wchar_t));
        if (!wbuf) return NULL;
    }
    int converted = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, (const char*)path_ptr, (int)path_len, wbuf, req_len);
    if (converted <= 0) {
        if (wbuf != stack_buf) free(wbuf);
        return NULL;
    }
    wbuf[converted] = L'\0';
    return wbuf;
}

static void luna_rt_free_wchar(wchar_t* wbuf, wchar_t* stack_buf) {
    if (wbuf && wbuf != stack_buf) {
        free(wbuf);
    }
}
#else
static char* luna_rt_path_to_cstr(const uint8_t* path_ptr, size_t path_len, char* stack_buf, size_t stack_cap) {
    char* cbuf = stack_buf;
    if (path_len + 1 > stack_cap) {
        cbuf = (char*)malloc(path_len + 1);
        if (!cbuf) return NULL;
    }
    memcpy(cbuf, path_ptr, path_len);
    cbuf[path_len] = '\0';
    return cbuf;
}

static void luna_rt_free_cstr(char* cbuf, char* stack_buf) {
    if (cbuf && cbuf != stack_buf) {
        free(cbuf);
    }
}
#endif

// --- Whole-File I/O ----------------------------------------------------------

#if defined(_WIN32)
typedef struct LunaRtWindowsReadContext {
    HANDLE handle;
} LunaRtWindowsReadContext;

static LunaRtReadResult luna_rt_read_next_windows(
    void* context,
    uint8_t* destination,
    size_t capacity,
    size_t* out_bytes
) {
    LunaRtWindowsReadContext* read_context = (LunaRtWindowsReadContext*)context;
    DWORD request = capacity > MAXDWORD ? MAXDWORD : (DWORD)capacity;
    DWORD bytes_read = 0;
    if (!ReadFile(read_context->handle, destination, request, &bytes_read, NULL)) {
        DWORD error = GetLastError();
        return error == ERROR_HANDLE_EOF ? LUNA_RT_READ_EOF : LUNA_RT_READ_ERROR;
    }
    if (bytes_read == 0) {
        return LUNA_RT_READ_EOF;
    }
    *out_bytes = (size_t)bytes_read;
    return LUNA_RT_READ_DATA;
}
#else
typedef struct LunaRtPosixReadContext {
    int fd;
} LunaRtPosixReadContext;

static LunaRtReadResult luna_rt_read_next_posix(
    void* context,
    uint8_t* destination,
    size_t capacity,
    size_t* out_bytes
) {
    LunaRtPosixReadContext* read_context = (LunaRtPosixReadContext*)context;
#ifdef SSIZE_MAX
    size_t request = capacity > (size_t)SSIZE_MAX ? (size_t)SSIZE_MAX : capacity;
#else
    size_t request = capacity > (size_t)PTRDIFF_MAX ? (size_t)PTRDIFF_MAX : capacity;
#endif
    ssize_t bytes_read = read(read_context->fd, destination, request);
    if (bytes_read > 0) {
        *out_bytes = (size_t)bytes_read;
        return LUNA_RT_READ_DATA;
    }
    if (bytes_read == 0) {
        return LUNA_RT_READ_EOF;
    }
    return errno == EINTR ? LUNA_RT_READ_INTERRUPTED : LUNA_RT_READ_ERROR;
}
#endif

int32_t __luna_read_file(
    const uint8_t*  path_ptr,
    size_t          path_len,
    uint8_t**       out_ptr,
    size_t*         out_len,
    size_t*         out_cap
) {
    if (!out_ptr || !out_len || !out_cap) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    *out_ptr = NULL;
    *out_len = 0;
    *out_cap = 0;

    // Defense against null path, zero length path, or embedded NUL bytes
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

    HANDLE hFile = CreateFileW(
        wpath,
        GENERIC_READ,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    if (hFile == INVALID_HANDLE_VALUE) {
        DWORD err = GetLastError();
        luna_rt_free_wchar(wpath, stack_wpath);
        if (err == ERROR_FILE_NOT_FOUND || err == ERROR_PATH_NOT_FOUND) {
            return LUNA_STATUS_NOT_FOUND;
        }
        if (err == ERROR_ACCESS_DENIED) {
            return LUNA_STATUS_PERMISSION_DENIED;
        }
        return LUNA_STATUS_IO_ERROR;
    }
    luna_rt_free_wchar(wpath, stack_wpath);

    LARGE_INTEGER fsize;
    size_t hint = 0;
    if (GetFileSizeEx(hFile, &fsize) && fsize.QuadPart > 0 && (uint64_t)fsize.QuadPart <= SIZE_MAX) {
        hint = (size_t)fsize.QuadPart;
    }
    LunaRtWindowsReadContext read_context = { hFile };
    int32_t status = luna_rt_read_all(
        luna_rt_read_next_windows,
        &read_context,
        hint,
        out_ptr,
        out_len,
        out_cap
    );
    CloseHandle(hFile);
    return status;

#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

    int fd;
    do {
        fd = open(cpath, O_RDONLY);
    } while (fd < 0 && errno == EINTR);
    if (fd < 0) {
        luna_rt_free_cstr(cpath, stack_cpath);
        if (errno == ENOENT) return LUNA_STATUS_NOT_FOUND;
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    luna_rt_free_cstr(cpath, stack_cpath);

    size_t hint = 0;
    struct stat st;
    if (fstat(fd, &st) == 0 && st.st_size > 0 && (uint64_t)st.st_size <= SIZE_MAX) {
        hint = (size_t)st.st_size;
    }
    LunaRtPosixReadContext read_context = { fd };
    int32_t status = luna_rt_read_all(
        luna_rt_read_next_posix,
        &read_context,
        hint,
        out_ptr,
        out_len,
        out_cap
    );
    close(fd);
    return status;
#endif
}

int32_t __luna_write_file(
    const uint8_t*  path_ptr,
    size_t          path_len,
    const uint8_t*  data_ptr,
    size_t          data_len
) {
    // Defense against null path, zero length path, or embedded NUL bytes
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    if (data_len > 0 && !data_ptr) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

    HANDLE hFile = CreateFileW(
        wpath,
        GENERIC_WRITE,
        0,
        NULL,
        CREATE_ALWAYS,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    if (hFile == INVALID_HANDLE_VALUE) {
        DWORD err = GetLastError();
        luna_rt_free_wchar(wpath, stack_wpath);
        if (err == ERROR_ACCESS_DENIED) {
            return LUNA_STATUS_PERMISSION_DENIED;
        }
        return LUNA_STATUS_IO_ERROR;
    }
    luna_rt_free_wchar(wpath, stack_wpath);

    size_t total_written = 0;
    while (total_written < data_len) {
        size_t remaining = data_len - total_written;
        DWORD chunk = (remaining > 0x40000000) ? 0x40000000 : (DWORD)remaining;
        DWORD bytes_written = 0;
        if (!WriteFile(hFile, data_ptr + total_written, chunk, &bytes_written, NULL)) {
            CloseHandle(hFile);
            return LUNA_STATUS_IO_ERROR;
        }
        if (bytes_written == 0) {
            CloseHandle(hFile);
            return LUNA_STATUS_IO_ERROR;
        }
        total_written += bytes_written;
    }

    return CloseHandle(hFile) ? LUNA_STATUS_OK : LUNA_STATUS_IO_ERROR;

#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }

    int fd = open(cpath, O_WRONLY | O_CREAT | O_TRUNC, 0666);
    if (fd < 0) {
        luna_rt_free_cstr(cpath, stack_cpath);
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    luna_rt_free_cstr(cpath, stack_cpath);

    size_t total_written = 0;
    while (total_written < data_len) {
        ssize_t n = write(fd, data_ptr + total_written, data_len - total_written);
        if (n < 0 && errno == EINTR) {
            continue;
        }
        if (n <= 0) {
            close(fd);
            return LUNA_STATUS_IO_ERROR;
        }
        total_written += (size_t)n;
    }

    return close(fd) == 0 ? LUNA_STATUS_OK : LUNA_STATUS_IO_ERROR;
#endif
}

// --- Stdin Stream Line Input -------------------------------------------------

int32_t __luna_read_line(
    uint8_t**       out_ptr,
    size_t*         out_len,
    size_t*         out_cap
) {
    if (!out_ptr || !out_len || !out_cap) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    *out_ptr = NULL;
    *out_len = 0;
    *out_cap = 0;

    size_t cap = 64;
    uint8_t* buf = (uint8_t*)__luna_alloc(cap, 1);
    size_t len = 0;

    while (1) {
        int c = fgetc(stdin);
        if (c == EOF) {
            if (len == 0) {
                // EOF before any characters read
                __luna_dealloc(buf, cap, 1);
                return LUNA_STATUS_EOF;
            }
            // EOF reached after reading some characters (no trailing newline)
            // Strip trailing '\r' if present
            if (len > 0 && buf[len - 1] == '\r') {
                len--;
            }
            *out_ptr = buf;
            *out_len = len;
            *out_cap = cap;
            return LUNA_STATUS_OK;
        }

        if (c == '\n') {
            // Line boundary reached! Strip trailing '\r' if present (CRLF normalization)
            if (len > 0 && buf[len - 1] == '\r') {
                len--;
            }
            *out_ptr = buf;
            *out_len = len;
            *out_cap = cap;
            return LUNA_STATUS_OK;
        }

        // Regular byte: ensure capacity
        if (len >= cap) {
            size_t new_cap = cap * 2;
            buf = (uint8_t*)__luna_realloc(buf, cap, 1, new_cap);
            cap = new_cap;
        }
        buf[len++] = (uint8_t)c;
    }
}
