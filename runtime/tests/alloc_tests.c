#include "luna/runtime/memory.h"
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

void __luna_test_alloc_fail_after(int64_t budget);
int64_t __luna_test_alloc_attempts(void);
int64_t __luna_test_alloc_live_blocks(void);

int main(void) {
    // These assertions are conformance checks even in Release builds.
    void* out = (void*)(uintptr_t)1;
    for (size_t align = 1; align <= 4096; align *= 2) {
        __luna_test_alloc_fail_after(0);
        int64_t attempts = __luna_test_alloc_attempts();
        assert(__luna_try_alloc(0, align, &out) == LUNA_ALLOC_OK);
        assert(__luna_is_zero_sentinel(out));
        assert((uintptr_t)out % align == 0);
        assert(__luna_test_alloc_attempts() == attempts);
        __luna_dealloc(out, 0, align);

        assert(__luna_try_alloc(17, align, &out) == LUNA_ALLOC_OUT_OF_MEMORY);
        assert(out == NULL);
        __luna_test_alloc_fail_after(-1);
        assert(__luna_try_realloc(NULL, 0, align, 17, &out) == LUNA_ALLOC_OK);
        assert((uintptr_t)out % align == 0);
        memset(out, 0xA7, 17);
        void* old = out;
        __luna_test_alloc_fail_after(0);
        assert(__luna_try_realloc(old, 17, align, 35, &out) == LUNA_ALLOC_OUT_OF_MEMORY);
        assert(out == NULL);
        assert(__luna_test_alloc_live_blocks() == 1);
        for (size_t i = 0; i < 17; ++i) assert(((unsigned char*)old)[i] == 0xA7);
        if (align > 1) {
            assert(__luna_try_realloc(old, 17, align, SIZE_MAX, &out) == LUNA_ALLOC_SIZE_OVERFLOW);
            assert(out == NULL);
        }
        __luna_test_alloc_fail_after(-1);
        assert(__luna_try_realloc(old, 17, align, 35, &out) == LUNA_ALLOC_OK);
        for (size_t i = 0; i < 17; ++i) assert(((unsigned char*)out)[i] == 0xA7);
        old = out;
        assert(__luna_try_realloc(old, 35, align, 7, &out) == LUNA_ALLOC_OK);
        for (size_t i = 0; i < 7; ++i) assert(((unsigned char*)out)[i] == 0xA7);
        old = out;
        __luna_test_alloc_fail_after(0);
        assert(__luna_try_realloc(old, 7, align, 0, &out) == LUNA_ALLOC_OK);
        assert(__luna_is_zero_sentinel(out));
        assert(__luna_test_alloc_live_blocks() == 0);
    }
    for (size_t align = 0; align <= 8192; align += 8192) {
        assert(__luna_try_alloc(0, align, &out) == LUNA_ALLOC_INVALID_ALIGN);
        assert(out == NULL);
    }
    assert(__luna_try_alloc(1, 3, &out) == LUNA_ALLOC_INVALID_ALIGN);
    assert(out == NULL);
    assert(__luna_try_alloc(SIZE_MAX, 64, &out) == LUNA_ALLOC_SIZE_OVERFLOW);
    assert(out == NULL);
    __luna_test_alloc_fail_after(1);
    assert(__luna_try_alloc(16, 8, &out) == LUNA_ALLOC_OK);
    void* old = out;
    assert(__luna_try_alloc(16, 8, &out) == LUNA_ALLOC_OUT_OF_MEMORY);
    assert(out == NULL);
    __luna_dealloc(old, 16, 8);
    assert(__luna_test_alloc_live_blocks() == 0);
    puts("fallible allocation: alignment, sentinel, overflow, failure, retry and ownership PASS");
    return 0;
}
