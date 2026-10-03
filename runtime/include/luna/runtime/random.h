#pragma once

#include "abi.h"

#ifdef __cplusplus
extern "C" {
#endif

int32_t  __luna_random_bytes(void* buf, uint64_t len);
uint64_t __luna_random_entropy(void);

#ifdef __cplusplus
}
#endif
