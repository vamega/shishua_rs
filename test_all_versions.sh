#!/usr/bin/env bash
set -euo pipefail

cargo test
cargo test --no-default-features
RUSTFLAGS="-C target-cpu=native" cargo test

if [[ -f shishua/shishua.h ]]; then
    run_c_binding_test() {
        local cflags=$1
        SHISHUA_C_DIR=shishua CFLAGS="$cflags" \
            cargo test --features=__intern_c_bindings
    }

    has_cpu_feature() {
        local feature=$1
        [[ -r /proc/cpuinfo ]] && grep -Eiq \
            "(^flags|^Features)[[:space:]]*:.*(^|[[:space:]])${feature}($|[[:space:]])" \
            /proc/cpuinfo
    }

    run_c_binding_test "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR"

    case "$(uname -m)" in
    x86_64 | amd64)
        run_c_binding_test \
            "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2"
        if has_cpu_feature avx2; then
            run_c_binding_test \
                "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell"
        else
            echo "Skipping AVX2 C bindings: CPU does not report avx2"
        fi
        ;;
    i386 | i486 | i586 | i686)
        if has_cpu_feature sse2; then
            run_c_binding_test \
                "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2"
        else
            echo "Skipping SSE2 C bindings: CPU does not report sse2"
        fi
        if has_cpu_feature avx2; then
            run_c_binding_test \
                "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell"
        else
            echo "Skipping AVX2 C bindings: CPU does not report avx2"
        fi
        ;;
    aarch64 | arm64)
        run_c_binding_test "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_NEON"
        ;;
    *)
        echo "Skipping architecture-specific C bindings on $(uname -m)"
        ;;
    esac
fi
