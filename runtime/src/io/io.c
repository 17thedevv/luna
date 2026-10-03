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
#include <dirent.h>
#include <limits.h>
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

// --- Console Stdio Stream ABI ------------------------------------------------

int64_t __luna_stdin_read(uint8_t* buf, size_t len) {
    if (!buf || len == 0) return 0;
    size_t n = fread(buf, 1, len, stdin);
    if (n == 0 && ferror(stdin)) return -1;
    return (int64_t)n;
}

int64_t __luna_stdout_write(const uint8_t* buf, size_t len) {
    if (!buf || len == 0) return 0;
    size_t n = fwrite(buf, 1, len, stdout);
    return (int64_t)n;
}

int32_t __luna_stdout_flush(void) {
    return fflush(stdout) == 0 ? 0 : -1;
}

int64_t __luna_stderr_write(const uint8_t* buf, size_t len) {
    if (!buf || len == 0) return 0;
    size_t n = fwrite(buf, 1, len, stderr);
    return (int64_t)n;
}

int32_t __luna_stderr_flush(void) {
    return fflush(stderr) == 0 ? 0 : -1;
}

// --- File Stream ABI ---------------------------------------------------------

typedef struct LunaFile {
#if defined(_WIN32)
    HANDLE handle;
#else
    int fd;
#endif
} LunaFile;

void* __luna_file_open(
    const uint8_t*  path_ptr,
    size_t          path_len,
    int32_t         read_mode,
    int32_t         write_mode,
    int32_t         create_mode,
    int32_t         truncate_mode,
    int32_t         append_mode
) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return NULL;
    }

#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) {
        return NULL;
    }

    DWORD access = 0;
    if (read_mode) access |= GENERIC_READ;
    if (write_mode) {
        if (append_mode) {
            access |= FILE_APPEND_DATA;
        } else {
            access |= GENERIC_WRITE;
        }
    }

    DWORD disposition = OPEN_EXISTING;
    if (create_mode) {
        if (truncate_mode) {
            disposition = CREATE_ALWAYS;
        } else {
            disposition = OPEN_ALWAYS;
        }
    } else {
        if (truncate_mode) {
            disposition = TRUNCATE_EXISTING;
        } else {
            disposition = OPEN_EXISTING;
        }
    }

    HANDLE h = CreateFileW(
        wpath,
        access,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        disposition,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    luna_rt_free_wchar(wpath, stack_wpath);
    if (h == INVALID_HANDLE_VALUE) {
        return NULL;
    }

    LunaFile* f = (LunaFile*)malloc(sizeof(LunaFile));
    if (!f) {
        CloseHandle(h);
        return NULL;
    }
    f->handle = h;
    return (void*)f;

#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) {
        return NULL;
    }

    int flags = 0;
    if (read_mode && write_mode) {
        flags = O_RDWR;
    } else if (write_mode) {
        flags = O_WRONLY;
    } else {
        flags = O_RDONLY;
    }

    if (create_mode) flags |= O_CREAT;
    if (truncate_mode) flags |= O_TRUNC;
    if (append_mode) flags |= O_APPEND;

    int fd;
    do {
        fd = open(cpath, flags, 0666);
    } while (fd < 0 && errno == EINTR);

    luna_rt_free_cstr(cpath, stack_cpath);
    if (fd < 0) {
        return NULL;
    }

    LunaFile* f = (LunaFile*)malloc(sizeof(LunaFile));
    if (!f) {
        close(fd);
        return NULL;
    }
    f->fd = fd;
    return (void*)f;
#endif
}

int64_t __luna_file_read(void* handle, uint8_t* buf, size_t len) {
    if (!handle || !buf || len == 0) return 0;
    LunaFile* f = (LunaFile*)handle;
#if defined(_WIN32)
    DWORD to_read = len > MAXDWORD ? MAXDWORD : (DWORD)len;
    DWORD bytes_read = 0;
    if (!ReadFile(f->handle, buf, to_read, &bytes_read, NULL)) {
        DWORD err = GetLastError();
        if (err == ERROR_HANDLE_EOF) return 0;
        return -1;
    }
    return (int64_t)bytes_read;
#else
    ssize_t n;
    do {
        n = read(f->fd, buf, len);
    } while (n < 0 && errno == EINTR);
    if (n < 0) return -1;
    return (int64_t)n;
#endif
}

int64_t __luna_file_write(void* handle, const uint8_t* buf, size_t len) {
    if (!handle || !buf || len == 0) return 0;
    LunaFile* f = (LunaFile*)handle;
#if defined(_WIN32)
    DWORD to_write = len > MAXDWORD ? MAXDWORD : (DWORD)len;
    DWORD bytes_written = 0;
    if (!WriteFile(f->handle, buf, to_write, &bytes_written, NULL)) {
        return -1;
    }
    return (int64_t)bytes_written;
#else
    ssize_t n;
    do {
        n = write(f->fd, buf, len);
    } while (n < 0 && errno == EINTR);
    if (n < 0) return -1;
    return (int64_t)n;
#endif
}

int64_t __luna_file_seek(void* handle, int64_t offset, int32_t whence) {
    if (!handle) return -1;
    LunaFile* f = (LunaFile*)handle;
#if defined(_WIN32)
    DWORD method = FILE_BEGIN;
    if (whence == 1) method = FILE_CURRENT;
    else if (whence == 2) method = FILE_END;

    LARGE_INTEGER dist;
    dist.QuadPart = offset;
    LARGE_INTEGER new_fp;
    if (!SetFilePointerEx(f->handle, dist, &new_fp, method)) {
        return -1;
    }
    return (int64_t)new_fp.QuadPart;
#else
    int posix_whence = SEEK_SET;
    if (whence == 1) posix_whence = SEEK_CUR;
    else if (whence == 2) posix_whence = SEEK_END;

    off_t res = lseek(f->fd, (off_t)offset, posix_whence);
    if (res == (off_t)-1) return -1;
    return (int64_t)res;
#endif
}

int32_t __luna_file_flush(void* handle) {
    if (!handle) return -1;
    LunaFile* f = (LunaFile*)handle;
#if defined(_WIN32)
    return FlushFileBuffers(f->handle) ? 0 : -1;
#else
    return fsync(f->fd) == 0 ? 0 : -1;
#endif
}

void __luna_file_close(void* handle) {
    if (!handle) return;
    LunaFile* f = (LunaFile*)handle;
#if defined(_WIN32)
    if (f->handle != INVALID_HANDLE_VALUE) {
        CloseHandle(f->handle);
        f->handle = INVALID_HANDLE_VALUE;
    }
#else
    if (f->fd >= 0) {
        close(f->fd);
        f->fd = -1;
    }
#endif
    free(f);
}

// --- File System / Directory ABI ---------------------------------------------

int32_t __luna_fs_create_dir(const uint8_t* path_ptr, size_t path_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) return LUNA_STATUS_INVALID_ARGUMENT;
    BOOL ok = CreateDirectoryW(wpath, NULL);
    luna_rt_free_wchar(wpath, stack_wpath);
    if (!ok) {
        DWORD err = GetLastError();
        if (err == ERROR_ALREADY_EXISTS) return -6; // AlreadyExists
        if (err == ERROR_ACCESS_DENIED) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    return LUNA_STATUS_OK;
#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) return LUNA_STATUS_INVALID_ARGUMENT;
    int res = mkdir(cpath, 0777);
    luna_rt_free_cstr(cpath, stack_cpath);
    if (res != 0) {
        if (errno == EEXIST) return -6;
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    return LUNA_STATUS_OK;
#endif
}

int32_t __luna_fs_remove_file(const uint8_t* path_ptr, size_t path_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) return LUNA_STATUS_INVALID_ARGUMENT;
    BOOL ok = DeleteFileW(wpath);
    luna_rt_free_wchar(wpath, stack_wpath);
    if (!ok) {
        DWORD err = GetLastError();
        if (err == ERROR_FILE_NOT_FOUND) return LUNA_STATUS_NOT_FOUND;
        if (err == ERROR_ACCESS_DENIED) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    return LUNA_STATUS_OK;
#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) return LUNA_STATUS_INVALID_ARGUMENT;
    int res = unlink(cpath);
    luna_rt_free_cstr(cpath, stack_cpath);
    if (res != 0) {
        if (errno == ENOENT) return LUNA_STATUS_NOT_FOUND;
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    return LUNA_STATUS_OK;
#endif
}

int32_t __luna_fs_remove_dir(const uint8_t* path_ptr, size_t path_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) return LUNA_STATUS_INVALID_ARGUMENT;
    BOOL ok = RemoveDirectoryW(wpath);
    luna_rt_free_wchar(wpath, stack_wpath);
    if (!ok) {
        DWORD err = GetLastError();
        if (err == ERROR_PATH_NOT_FOUND || err == ERROR_FILE_NOT_FOUND) return LUNA_STATUS_NOT_FOUND;
        if (err == ERROR_ACCESS_DENIED) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    return LUNA_STATUS_OK;
#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) return LUNA_STATUS_INVALID_ARGUMENT;
    int res = rmdir(cpath);
    luna_rt_free_cstr(cpath, stack_cpath);
    if (res != 0) {
        if (errno == ENOENT) return LUNA_STATUS_NOT_FOUND;
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    return LUNA_STATUS_OK;
#endif
}

int32_t __luna_fs_metadata(
    const uint8_t*  path_ptr,
    size_t          path_len,
    uint64_t*       out_size,
    int32_t*        out_is_dir
) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    if (!out_size || !out_is_dir) return LUNA_STATUS_INVALID_ARGUMENT;
    *out_size = 0;
    *out_is_dir = 0;

#if defined(_WIN32)
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) return LUNA_STATUS_INVALID_ARGUMENT;
    WIN32_FILE_ATTRIBUTE_DATA data;
    BOOL ok = GetFileAttributesExW(wpath, GetFileExInfoStandard, &data);
    luna_rt_free_wchar(wpath, stack_wpath);
    if (!ok) {
        DWORD err = GetLastError();
        if (err == ERROR_FILE_NOT_FOUND || err == ERROR_PATH_NOT_FOUND) return LUNA_STATUS_NOT_FOUND;
        if (err == ERROR_ACCESS_DENIED) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    if (data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
        *out_is_dir = 1;
        *out_size = 0;
    } else {
        *out_is_dir = 0;
        ULARGE_INTEGER sz;
        sz.LowPart = data.nFileSizeLow;
        sz.HighPart = data.nFileSizeHigh;
        *out_size = (uint64_t)sz.QuadPart;
    }
    return LUNA_STATUS_OK;
#else
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) return LUNA_STATUS_INVALID_ARGUMENT;
    struct stat st;
    int res = stat(cpath, &st);
    luna_rt_free_cstr(cpath, stack_cpath);
    if (res != 0) {
        if (errno == ENOENT) return LUNA_STATUS_NOT_FOUND;
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    *out_is_dir = S_ISDIR(st.st_mode) ? 1 : 0;
    *out_size = (uint64_t)st.st_size;
    return LUNA_STATUS_OK;
#endif
}

#if defined(_WIN32)
typedef struct LunaDirHandle {
    HANDLE hFind;
    WIN32_FIND_DATAW findData;
    int has_entry;
} LunaDirHandle;

void* __luna_fs_opendir(const uint8_t* path_ptr, size_t path_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return NULL;
    }
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) return NULL;

    size_t wlen = wcslen(wpath);
    wchar_t pattern[1024];
    if (wlen + 3 >= sizeof(pattern) / sizeof(wchar_t)) {
        luna_rt_free_wchar(wpath, stack_wpath);
        return NULL;
    }
    wcscpy(pattern, wpath);
    if (wlen > 0 && pattern[wlen - 1] != L'\\' && pattern[wlen - 1] != L'/') {
        pattern[wlen] = L'\\';
        pattern[wlen + 1] = L'*';
        pattern[wlen + 2] = L'\0';
    } else {
        pattern[wlen] = L'*';
        pattern[wlen + 1] = L'\0';
    }
    luna_rt_free_wchar(wpath, stack_wpath);

    LunaDirHandle* dir = (LunaDirHandle*)malloc(sizeof(LunaDirHandle));
    if (!dir) return NULL;

    dir->hFind = FindFirstFileW(pattern, &dir->findData);
    if (dir->hFind == INVALID_HANDLE_VALUE) {
        free(dir);
        return NULL;
    }
    dir->has_entry = 1;
    return (void*)dir;
}

int32_t __luna_fs_readdir(void* handle, uint8_t* out_buf, size_t max_len, size_t* out_len, int32_t* out_is_dir) {
    if (!handle || !out_buf || max_len == 0 || !out_len || !out_is_dir) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    LunaDirHandle* dir = (LunaDirHandle*)handle;
    while (1) {
        if (!dir->has_entry) {
            if (!FindNextFileW(dir->hFind, &dir->findData)) {
                return 1; // EOF (no more entries)
            }
        }
        dir->has_entry = 0;

        if (wcscmp(dir->findData.cFileName, L".") == 0 || wcscmp(dir->findData.cFileName, L"..") == 0) {
            continue;
        }

        int utf8_bytes = WideCharToMultiByte(CP_UTF8, 0, dir->findData.cFileName, -1, (char*)out_buf, (int)max_len, NULL, NULL);
        if (utf8_bytes <= 0) {
            return LUNA_STATUS_IO_ERROR;
        }
        *out_len = (size_t)(utf8_bytes - 1);
        *out_is_dir = (dir->findData.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? 1 : 0;
        return 0; // Success
    }
}

void __luna_fs_closedir(void* handle) {
    if (handle) {
        LunaDirHandle* dir = (LunaDirHandle*)handle;
        if (dir->hFind != INVALID_HANDLE_VALUE) {
            FindClose(dir->hFind);
        }
        free(dir);
    }
}

int32_t __luna_fs_canonicalize(const uint8_t* path_ptr, size_t path_len, uint8_t* out_buf, size_t max_len, size_t* out_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL || !out_buf || max_len == 0 || !out_len) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    wchar_t stack_wpath[512];
    wchar_t* wpath = luna_rt_path_to_wchar(path_ptr, path_len, stack_wpath, sizeof(stack_wpath) / sizeof(wchar_t));
    if (!wpath) return LUNA_STATUS_INVALID_ARGUMENT;

    wchar_t full_path[1024];
    DWORD res = GetFullPathNameW(wpath, sizeof(full_path) / sizeof(wchar_t), full_path, NULL);
    luna_rt_free_wchar(wpath, stack_wpath);
    if (res == 0 || res >= sizeof(full_path) / sizeof(wchar_t)) {
        return LUNA_STATUS_IO_ERROR;
    }

    int utf8_bytes = WideCharToMultiByte(CP_UTF8, 0, full_path, -1, (char*)out_buf, (int)max_len, NULL, NULL);
    if (utf8_bytes <= 0) {
        return LUNA_STATUS_IO_ERROR;
    }
    *out_len = (size_t)(utf8_bytes - 1);
    return LUNA_STATUS_OK;
}
#else
void* __luna_fs_opendir(const uint8_t* path_ptr, size_t path_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL) {
        return NULL;
    }
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) return NULL;
    DIR* d = opendir(cpath);
    luna_rt_free_cstr(cpath, stack_cpath);
    return (void*)d;
}

int32_t __luna_fs_readdir(void* handle, uint8_t* out_buf, size_t max_len, size_t* out_len, int32_t* out_is_dir) {
    if (!handle || !out_buf || max_len == 0 || !out_len || !out_is_dir) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    DIR* d = (DIR*)handle;
    struct dirent* entry;
    while ((entry = readdir(d)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        size_t nlen = strlen(entry->d_name);
        if (nlen >= max_len) {
            nlen = max_len - 1;
        }
        memcpy(out_buf, entry->d_name, nlen);
        out_buf[nlen] = '\0';
        *out_len = nlen;
        *out_is_dir = (entry->d_type == DT_DIR) ? 1 : 0;
        return 0; // Success
    }
    return 1; // EOF
}

void __luna_fs_closedir(void* handle) {
    if (handle) {
        closedir((DIR*)handle);
    }
}

int32_t __luna_fs_canonicalize(const uint8_t* path_ptr, size_t path_len, uint8_t* out_buf, size_t max_len, size_t* out_len) {
    if (!path_ptr || path_len == 0 || memchr(path_ptr, 0, path_len) != NULL || !out_buf || max_len == 0 || !out_len) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    char stack_cpath[1024];
    char* cpath = luna_rt_path_to_cstr(path_ptr, path_len, stack_cpath, sizeof(stack_cpath));
    if (!cpath) return LUNA_STATUS_INVALID_ARGUMENT;

    char resolved[PATH_MAX];
    char* res = realpath(cpath, resolved);
    luna_rt_free_cstr(cpath, stack_cpath);
    if (!res) {
        if (errno == ENOENT) return LUNA_STATUS_NOT_FOUND;
        if (errno == EACCES) return LUNA_STATUS_PERMISSION_DENIED;
        return LUNA_STATUS_IO_ERROR;
    }
    size_t rlen = strlen(resolved);
    if (rlen >= max_len) {
        return LUNA_STATUS_INVALID_ARGUMENT;
    }
    memcpy(out_buf, resolved, rlen);
    out_buf[rlen] = '\0';
    *out_len = rlen;
    return LUNA_STATUS_OK;
}
#endif

// --- Process Execution ABI ---------------------------------------------------

typedef struct LunaChild {
#if defined(_WIN32)
    HANDLE hProcess;
    HANDLE hThread;
    DWORD dwProcessId;
#else
    pid_t pid;
#endif
} LunaChild;

void* __luna_process_spawn(const uint8_t* cmd_ptr, size_t cmd_len) {
    if (!cmd_ptr || cmd_len == 0 || memchr(cmd_ptr, 0, cmd_len) != NULL) {
        return NULL;
    }

#if defined(_WIN32)
    wchar_t stack_wcmd[1024];
    wchar_t* wcmd = luna_rt_path_to_wchar(cmd_ptr, cmd_len, stack_wcmd, sizeof(stack_wcmd) / sizeof(wchar_t));
    if (!wcmd) return NULL;

    STARTUPINFOW si;
    PROCESS_INFORMATION pi;
    memset(&si, 0, sizeof(si));
    si.cb = sizeof(si);
    memset(&pi, 0, sizeof(pi));

    BOOL success = CreateProcessW(
        NULL,
        wcmd,
        NULL,
        NULL,
        FALSE,
        0,
        NULL,
        NULL,
        &si,
        &pi
    );

    luna_rt_free_wchar(wcmd, stack_wcmd);
    if (!success) {
        return NULL;
    }

    LunaChild* child = (LunaChild*)malloc(sizeof(LunaChild));
    if (!child) {
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
        return NULL;
    }
    child->hProcess = pi.hProcess;
    child->hThread = pi.hThread;
    child->dwProcessId = pi.dwProcessId;
    return (void*)child;

#else
    char stack_ccmd[1024];
    char* ccmd = luna_rt_path_to_cstr(cmd_ptr, cmd_len, stack_ccmd, sizeof(stack_ccmd));
    if (!ccmd) return NULL;

    pid_t pid = fork();
    if (pid < 0) {
        luna_rt_free_cstr(ccmd, stack_ccmd);
        return NULL;
    }
    if (pid == 0) {
        execl("/bin/sh", "sh", "-c", ccmd, (char*)NULL);
        _exit(127);
    }
    luna_rt_free_cstr(ccmd, stack_ccmd);

    LunaChild* child = (LunaChild*)malloc(sizeof(LunaChild));
    if (!child) return NULL;
    child->pid = pid;
    return (void*)child;
#endif
}

int32_t __luna_process_wait(void* handle, int32_t* out_exit_code) {
    if (!handle || !out_exit_code) return -1;
    LunaChild* child = (LunaChild*)handle;
#if defined(_WIN32)
    DWORD wait_res = WaitForSingleObject(child->hProcess, INFINITE);
    if (wait_res != WAIT_OBJECT_0) return -1;
    DWORD exit_code = 0;
    if (!GetExitCodeProcess(child->hProcess, &exit_code)) return -1;
    *out_exit_code = (int32_t)exit_code;
    return 0;
#else
    int status = 0;
    if (waitpid(child->pid, &status, 0) < 0) return -1;
    if (WIFEXITED(status)) {
        *out_exit_code = (int32_t)WEXITSTATUS(status);
    } else {
        *out_exit_code = -1;
    }
    return 0;
#endif
}

int32_t __luna_process_kill(void* handle) {
    if (!handle) return -1;
    LunaChild* child = (LunaChild*)handle;
#if defined(_WIN32)
    return TerminateProcess(child->hProcess, 1) ? 0 : -1;
#else
    return kill(child->pid, SIGKILL) == 0 ? 0 : -1;
#endif
}

uint32_t __luna_process_id(void* handle) {
    if (!handle) return 0;
    LunaChild* child = (LunaChild*)handle;
#if defined(_WIN32)
    return (uint32_t)child->dwProcessId;
#else
    return (uint32_t)child->pid;
#endif
}

void __luna_process_close(void* handle) {
    if (!handle) return;
    LunaChild* child = (LunaChild*)handle;
#if defined(_WIN32)
    if (child->hProcess != INVALID_HANDLE_VALUE && child->hProcess != NULL) {
        CloseHandle(child->hProcess);
    }
    if (child->hThread != INVALID_HANDLE_VALUE && child->hThread != NULL) {
        CloseHandle(child->hThread);
    }
#endif
    free(child);
}

int32_t __luna_process_output(
    const uint8_t* cmd_ptr,
    size_t cmd_len,
    int32_t* out_exit_code,
    uint8_t** out_out_ptr,
    size_t* out_out_len,
    size_t* out_out_cap,
    uint8_t** out_err_ptr,
    size_t* out_err_len,
    size_t* out_err_cap
) {
    if (!cmd_ptr || cmd_len == 0 || !out_exit_code || !out_out_ptr || !out_out_len || !out_out_cap || !out_err_ptr || !out_err_len || !out_err_cap) {
        return -1;
    }
    *out_out_ptr = NULL; *out_out_len = 0; *out_out_cap = 0;
    *out_err_ptr = NULL; *out_err_len = 0; *out_err_cap = 0;
    *out_exit_code = -1;

#if defined(_WIN32)
    wchar_t stack_wcmd[1024];
    wchar_t* wcmd = luna_rt_path_to_wchar(cmd_ptr, cmd_len, stack_wcmd, sizeof(stack_wcmd) / sizeof(wchar_t));
    if (!wcmd) return -1;

    SECURITY_ATTRIBUTES sa;
    sa.nLength = sizeof(sa);
    sa.bInheritHandle = TRUE;
    sa.lpSecurityDescriptor = NULL;

    HANDLE hOutRead = NULL, hOutWrite = NULL;
    HANDLE hErrRead = NULL, hErrWrite = NULL;

    if (!CreatePipe(&hOutRead, &hOutWrite, &sa, 0)) {
        luna_rt_free_wchar(wcmd, stack_wcmd);
        return -1;
    }
    SetHandleInformation(hOutRead, HANDLE_FLAG_INHERIT, 0);

    if (!CreatePipe(&hErrRead, &hErrWrite, &sa, 0)) {
        CloseHandle(hOutRead); CloseHandle(hOutWrite);
        luna_rt_free_wchar(wcmd, stack_wcmd);
        return -1;
    }
    SetHandleInformation(hErrRead, HANDLE_FLAG_INHERIT, 0);

    STARTUPINFOW si;
    PROCESS_INFORMATION pi;
    memset(&si, 0, sizeof(si));
    si.cb = sizeof(si);
    si.dwFlags |= STARTF_USESTDHANDLES;
    si.hStdOutput = hOutWrite;
    si.hStdError = hErrWrite;
    si.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    memset(&pi, 0, sizeof(pi));

    BOOL success = CreateProcessW(NULL, wcmd, NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi);
    luna_rt_free_wchar(wcmd, stack_wcmd);
    CloseHandle(hOutWrite);
    CloseHandle(hErrWrite);

    if (!success) {
        CloseHandle(hOutRead);
        CloseHandle(hErrRead);
        return -1;
    }

    size_t out_cap = 256;
    uint8_t* out_buf = (uint8_t*)__luna_alloc(out_cap, 1);
    size_t out_len = 0;
    while (1) {
        if (out_len + 256 > out_cap) {
            size_t new_cap = out_cap * 2;
            out_buf = (uint8_t*)__luna_realloc(out_buf, out_cap, 1, new_cap);
            out_cap = new_cap;
        }
        DWORD bytes_read = 0;
        if (!ReadFile(hOutRead, out_buf + out_len, 256, &bytes_read, NULL) || bytes_read == 0) {
            break;
        }
        out_len += bytes_read;
    }
    CloseHandle(hOutRead);
    *out_out_ptr = out_buf;
    *out_out_len = out_len;
    *out_out_cap = out_cap;

    size_t err_cap = 256;
    uint8_t* err_buf = (uint8_t*)__luna_alloc(err_cap, 1);
    size_t err_len = 0;
    while (1) {
        if (err_len + 256 > err_cap) {
            size_t new_cap = err_cap * 2;
            err_buf = (uint8_t*)__luna_realloc(err_buf, err_cap, 1, new_cap);
            err_cap = new_cap;
        }
        DWORD bytes_read = 0;
        if (!ReadFile(hErrRead, err_buf + err_len, 256, &bytes_read, NULL) || bytes_read == 0) {
            break;
        }
        err_len += bytes_read;
    }
    CloseHandle(hErrRead);
    *out_err_ptr = err_buf;
    *out_err_len = err_len;
    *out_err_cap = err_cap;

    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD exit_code = 0;
    GetExitCodeProcess(pi.hProcess, &exit_code);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    *out_exit_code = (int32_t)exit_code;
    return 0;

#else
    int out_pipe[2];
    int err_pipe[2];
    if (pipe(out_pipe) < 0) return -1;
    if (pipe(err_pipe) < 0) { close(out_pipe[0]); close(out_pipe[1]); return -1; }

    char stack_ccmd[1024];
    char* ccmd = luna_rt_path_to_cstr(cmd_ptr, cmd_len, stack_ccmd, sizeof(stack_ccmd));
    if (!ccmd) {
        close(out_pipe[0]); close(out_pipe[1]);
        close(err_pipe[0]); close(err_pipe[1]);
        return -1;
    }

    pid_t pid = fork();
    if (pid < 0) {
        luna_rt_free_cstr(ccmd, stack_ccmd);
        close(out_pipe[0]); close(out_pipe[1]);
        close(err_pipe[0]); close(err_pipe[1]);
        return -1;
    }

    if (pid == 0) {
        close(out_pipe[0]);
        close(err_pipe[0]);
        dup2(out_pipe[1], STDOUT_FILENO);
        dup2(err_pipe[1], STDERR_FILENO);
        close(out_pipe[1]);
        close(err_pipe[1]);
        execl("/bin/sh", "sh", "-c", ccmd, (char*)NULL);
        _exit(127);
    }

    luna_rt_free_cstr(ccmd, stack_ccmd);
    close(out_pipe[1]);
    close(err_pipe[1]);

    size_t out_cap = 256;
    uint8_t* out_buf = (uint8_t*)__luna_alloc(out_cap, 1);
    size_t out_len = 0;
    while (1) {
        if (out_len + 256 > out_cap) {
            size_t new_cap = out_cap * 2;
            out_buf = (uint8_t*)__luna_realloc(out_buf, out_cap, 1, new_cap);
            out_cap = new_cap;
        }
        ssize_t n = read(out_pipe[0], out_buf + out_len, 256);
        if (n <= 0) break;
        out_len += (size_t)n;
    }
    close(out_pipe[0]);
    *out_out_ptr = out_buf;
    *out_out_len = out_len;
    *out_out_cap = out_cap;

    size_t err_cap = 256;
    uint8_t* err_buf = (uint8_t*)__luna_alloc(err_cap, 1);
    size_t err_len = 0;
    while (1) {
        if (err_len + 256 > err_cap) {
            size_t new_cap = err_cap * 2;
            err_buf = (uint8_t*)__luna_realloc(err_buf, err_cap, 1, new_cap);
            err_cap = new_cap;
        }
        ssize_t n = read(err_pipe[0], err_buf + err_len, 256);
        if (n <= 0) break;
        err_len += (size_t)n;
    }
    close(err_pipe[0]);
    *out_err_ptr = err_buf;
    *out_err_len = err_len;
    *out_err_cap = err_cap;

    int status = 0;
    waitpid(pid, &status, 0);
    *out_exit_code = WIFEXITED(status) ? (int32_t)WEXITSTATUS(status) : -1;
    return 0;
#endif
}
