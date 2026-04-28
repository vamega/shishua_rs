#if defined(_MSC_VER)
#include <malloc.h>
#else
#include <stdlib.h>
#endif

#include "shishua.h"

prng_state* shishua_bindings_init(uint64_t* seed) {
    prng_state* state;
#if defined(_MSC_VER)
    state = _aligned_malloc(sizeof(prng_state), 32);
    if (state == NULL) {
        return NULL;
    }
#else
    if (posix_memalign((void**)&state, 32, sizeof(prng_state)) != 0) {
        return NULL;
    }
#endif
    prng_init(state, seed);
    return state;
}

void shishua_bindings_destroy(prng_state* state) {
#if defined(_MSC_VER)
    _aligned_free(state);
#else
    free(state);
#endif
}

void shishua_bindings_generate(prng_state* state, uint8_t* buffer, size_t size) {
    prng_gen(state, buffer, size);
}
