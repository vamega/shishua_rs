Straightforward port of [shishua](https://github.com/espadrine/shishua) from C to Rust.

This crate builds on stable Rust. `ShiShuAState::new` uses runtime dispatch on
x86/x86_64, preferring AVX2, then SSSE3, then SSE2, then the scalar fallback.
On aarch64 it uses the NEON backend. The explicit constructors (`new_scalar`,
`new_sse2`, `new_ssse3`, `new_avx2`, and `new_neon` on supported targets) are
available for benchmarks.

```sh
cargo bench
```

To run the local test matrix, including no-default-feature checks and available
C backend comparisons:

```sh
cargo run -p xtask -- test-all
```

The benchmark can compare against the original C implementation when the C
source is available locally. For example, to compare against the C SSE2 path:

```sh
git clone --depth 1 https://github.com/espadrine/shishua.git shishua
SHISHUA_C_DIR=shishua \
  CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2" \
  cargo bench --features=__intern_c_bindings
```

To compare against the C SSSE3 path, use the same C target with SSSE3 enabled:

```sh
SHISHUA_C_DIR=shishua \
  CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -mssse3 -mno-avx -mno-avx2" \
  cargo bench --features=__intern_c_bindings
```
