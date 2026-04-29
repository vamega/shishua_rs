Straightforward port of [shishua](https://github.com/espadrine/shishua) from C to Rust.

This crate builds on stable Rust. `ShiShuAState::new` uses runtime dispatch on
x86/x86_64, preferring AVX2, then SSE2, then the scalar fallback. On aarch64 it
uses the NEON backend. The explicit constructors (`new_scalar`, `new_sse2`,
`new_avx2`, and `new_neon` on supported targets) are available for benchmarks.

```sh
cargo bench
```

With Nix, enter the dev shell to use a Nix-provided Rust toolchain on either
aarch64 or x86_64:

```sh
nix develop
build-native
cargo test
```

The default Nix benchmark helper compares the Rust runtime-selected backend
against a C implementation built by the shell's default C compiler. On x86_64
it uses C AVX2 when the CPU advertises AVX2, otherwise C SSE2. On aarch64 the
default helper uses the C scalar target so the default dev shell does not pull
in Clang. Helpers use Nix-provided `rustc` and `cargo`, and build Rust with
`-C target-cpu=native` unless `RUSTFLAGS` is already set:

```sh
nix develop
bench-native
```

The same build and benchmark helpers can be launched without an interactive
shell:

```sh
nix run .#build-native -- --release
nix run .#bench-native
```

Explicit C comparison targets are also available:

```sh
nix run .#bench-x64-sse2   # x86_64 only
nix run .#bench-x64-avx2   # x86_64 with AVX2 only
nix run .#bench-c-scalar
```

The Clang-built ARM NEON comparison is kept out of the default dev shell to keep
the default closure smaller. Run it directly, or enter the explicit Clang shell:

```sh
nix run .#bench-arm-neon   # aarch64/arm64 only
nix develop .#clang
bench-arm-neon
```

The benchmark can compare against the original C implementation when the
`shishua` C source is available locally. For example, without the Nix helper:

```sh
git clone --depth 1 https://github.com/espadrine/shishua.git shishua
SHISHUA_C_DIR=shishua \
  CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2" \
  cargo bench --features=__intern_c_bindings
```
