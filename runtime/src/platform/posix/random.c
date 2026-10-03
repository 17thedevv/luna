// =============================================================================
// runtime/src/platform/posix/random.c
//
// Luna Runtime — Random Entropy ABI (POSIX Platform Implementation)
// =============================================================================

#include "luna/runtime/random.h"
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <sys/types.h>

int32_t __luna_random_bytes(void* buf, uint64_t len) {
    if (!buf || len == 0) return 0;

    int fd = open("/dev/urandom", O_RDONLY);
    if (fd < 0) {
        return -1;
    }

    uint8_t* p = (uint8_t*)buf;
    uint64_t remaining = len;
    while (remaining > 0) {
        ssize_t n = read(fd, p, remaining);
        if (n <= 0) {
            close(fd);
            return -2;
        }
        p += n;
        remaining -= (uint64_t)n;
    }

    close(fd);
    return 0;
}

uint64_t __luna_random_entropy(void) {
    uint64_t entropy = 0;
    if (__luna_random_bytes(&entropy, sizeof(entropy)) == 0 && entropy != 0) {
        return entropy;
    }

    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);

    uint64_t pid = (uint64_t)getpid();
    uint64_t t = ((uint64_t)ts.tv_sec * 1000000000ULL) + (uint64_t)ts.tv_nsec;

    uint64_t z = (t ^ (pid << 32)) + 0x9E3779B97F4A7C15ULL;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}
