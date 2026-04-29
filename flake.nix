{
  description = "Development shell for the shishua Rust port";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      systems = [
        "aarch64-linux"
        "x86_64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f system (import nixpkgs { inherit system; })
        );
    in
    {
      packages = forEachSystem (
        system: pkgs:
        let
          defaultRuntimeInputs = with pkgs; [
            cargo
            coreutils
            gnugrep
            rustc
            stdenv.cc
          ];

          clangRuntimeInputs = with pkgs; [
            cargo
            clang
            coreutils
            gnugrep
            rustc
          ];

          commonPreamble = ''
            set -euo pipefail

            print_toolchain() {
              echo "rustc: $(rustc --version)"
              echo "cc: $($CC --version | head -n 1)"
              echo "RUSTFLAGS: $RUSTFLAGS"
            }

            require_shishua_c() {
              export SHISHUA_C_DIR="''${SHISHUA_C_DIR:-shishua}"
              if [ ! -f "$SHISHUA_C_DIR/shishua.h" ]; then
                echo "Missing $SHISHUA_C_DIR/shishua.h. Clone https://github.com/espadrine/shishua into ./shishua or set SHISHUA_C_DIR." >&2
                exit 1
              fi
              echo "SHISHUA_C_DIR: $SHISHUA_C_DIR"
            }
          '';

          mkCargoCommand =
            {
              name,
              runtimeInputs ? defaultRuntimeInputs,
              text,
            }:
            pkgs.writeShellApplication {
              inherit name runtimeInputs;
              text = commonPreamble + text;
            };

          mkCBenchmark =
            {
              name,
              cflags,
              requiredArch ? null,
              runtimeInputs ? defaultRuntimeInputs,
              cc ? "cc",
            }:
            mkCargoCommand {
              inherit name runtimeInputs;
              text = ''
              ${pkgs.lib.optionalString (requiredArch != null) ''
              case "$(uname -m)" in
                ${requiredArch}) ;;
                *)
                  echo "${name} requires ${requiredArch}." >&2
                  exit 1
                  ;;
              esac
              ''}

              require_shishua_c
              export CC="''${CC:-${cc}}"
              export CFLAGS="''${CFLAGS:-${cflags}}"
              export RUSTFLAGS="''${RUSTFLAGS:--C target-cpu=native}"

              print_toolchain
              echo "CFLAGS: $CFLAGS"

              cargo bench --features=__intern_c_bindings --bench bench -- "$@"
              '';
            };

          buildNative = mkCargoCommand {
            name = "build-native";
            text = ''
            export CC="''${CC:-cc}"
            export RUSTFLAGS="''${RUSTFLAGS:--C target-cpu=native}"

            print_toolchain

            cargo build "$@"
            '';
          };

          benchNative = mkCargoCommand {
            name = "bench-native";
            text = ''
            require_shishua_c
            export CC="''${CC:-cc}"
            export RUSTFLAGS="''${RUSTFLAGS:--C target-cpu=native}"

            if [ -z "''${CFLAGS:-}" ]; then
              case "$(uname -m)" in
                aarch64|arm64)
                  export CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR"
                  echo "Using the C scalar target for default native benchmarking; use bench-arm-neon for Clang-built C NEON." >&2
                  ;;
                x86_64|amd64)
                  if [ -r /proc/cpuinfo ] && grep -qw avx2 /proc/cpuinfo; then
                    export CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell"
                  elif command -v sysctl >/dev/null 2>&1 && sysctl -n machdep.cpu.leaf7_features 2>/dev/null | grep -qw AVX2; then
                    export CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell"
                  else
                    export CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2"
                  fi
                  ;;
                *)
                  export CFLAGS="-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR"
                  ;;
              esac
            fi

            print_toolchain
            echo "CFLAGS: $CFLAGS"

            cargo bench --features=__intern_c_bindings --bench bench -- "$@"
            '';
          };

          benchArmNeon = mkCBenchmark {
            name = "bench-arm-neon";
            requiredArch = "aarch64|arm64";
            runtimeInputs = clangRuntimeInputs;
            cc = "clang";
            cflags = "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_NEON";
          };

          benchX64Sse2 = mkCBenchmark {
            name = "bench-x64-sse2";
            requiredArch = "x86_64|amd64";
            cflags = "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2";
          };

          benchX64Avx2 = mkCBenchmark {
            name = "bench-x64-avx2";
            requiredArch = "x86_64|amd64";
            cflags = "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell";
          };

          benchCScalar = mkCBenchmark {
            name = "bench-c-scalar";
            cflags = "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR";
          };
        in
        {
          "bench-arm-neon" = benchArmNeon;
          "bench-c-scalar" = benchCScalar;
          "bench-native" = benchNative;
          "bench-x64-avx2" = benchX64Avx2;
          "bench-x64-sse2" = benchX64Sse2;
          "build-native" = buildNative;
          default = benchNative;
        }
      );

      apps = forEachSystem (system: pkgs: {
        "bench-arm-neon" = {
          type = "app";
          program = "${self.packages.${system}."bench-arm-neon"}/bin/bench-arm-neon";
        };
        "bench-c-scalar" = {
          type = "app";
          program = "${self.packages.${system}."bench-c-scalar"}/bin/bench-c-scalar";
        };
        "bench-native" = {
          type = "app";
          program = "${self.packages.${system}."bench-native"}/bin/bench-native";
        };
        "bench-x64-avx2" = {
          type = "app";
          program = "${self.packages.${system}."bench-x64-avx2"}/bin/bench-x64-avx2";
        };
        "bench-x64-sse2" = {
          type = "app";
          program = "${self.packages.${system}."bench-x64-sse2"}/bin/bench-x64-sse2";
        };
        "build-native" = {
          type = "app";
          program = "${self.packages.${system}."build-native"}/bin/build-native";
        };
        default = self.apps.${system}."bench-native";
      });

      devShells = forEachSystem (
        system: pkgs:
        let
          defaultShellPackages = with pkgs; [
            cargo
            clippy
            pkg-config
            rust-analyzer
            rustc
            rustfmt
            self.packages.${system}."bench-c-scalar"
            self.packages.${system}."bench-native"
            self.packages.${system}."bench-x64-avx2"
            self.packages.${system}."bench-x64-sse2"
            self.packages.${system}."build-native"
          ];
        in
        {
          default = pkgs.mkShell {
            packages = defaultShellPackages;

            RUST_BACKTRACE = "1";

            shellHook = ''
              echo "Rust toolchain: $(rustc --version)"
              echo "Build: build-native"
              echo "Benchmark: bench-native"
              echo "Explicit C targets: bench-x64-sse2, bench-x64-avx2, bench-c-scalar"
              echo "Clang/ARM NEON shell: nix develop .#clang"
            '';
          };

          clang = pkgs.mkShell {
            packages =
              defaultShellPackages
              ++ (with pkgs; [
                clang
                self.packages.${system}."bench-arm-neon"
              ]);

            CC = "clang";
            RUST_BACKTRACE = "1";

            shellHook = ''
              echo "Rust toolchain: $(rustc --version)"
              echo "Build: build-native"
              echo "Benchmark: bench-native"
              echo "Clang C target: bench-arm-neon"
            '';
          };
        }
      );
    };
}
