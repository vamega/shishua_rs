#!/usr/bin/env bash
set -euo pipefail

cargo test
cargo test --no-default-features
RUSTFLAGS="-C target-cpu=native" cargo test

if [[ -f shishua/shishua.h ]]; then
    SHISHUA_C_DIR=shishua CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR" \
        cargo test --features=__intern_c_bindings
    SHISHUA_C_DIR=shishua \
        CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2" \
        cargo test --features=__intern_c_bindings
    SHISHUA_C_DIR=shishua \
        CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell" \
        cargo test --features=__intern_c_bindings
fi
