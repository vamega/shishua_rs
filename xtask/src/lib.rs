use std::{
    env, error::Error, fmt, path::Path, path::PathBuf, process::Command,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuFeatures {
    pub sse2: bool,
    pub avx2: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoStep {
    args: Vec<String>,
    env: Vec<(String, String)>,
}

impl CargoStep {
    pub fn new<const N: usize>(args: [&str; N]) -> Self {
        Self {
            args: args.into_iter().map(str::to_owned).collect(),
            env: Vec::new(),
        }
    }

    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    fn run(&self, workspace_root: &Path) -> Result<(), Box<dyn Error>> {
        println!("+ {self}");

        let mut command = Command::new("cargo");
        command.current_dir(workspace_root).args(&self.args);
        for (key, value) in &self.env {
            command.env(key, value);
        }

        let status = command.status()?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("command failed with {status}: {self}").into())
        }
    }
}

impl fmt::Display for CargoStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in &self.env {
            write!(f, "{key}={value:?} ")?;
        }
        write!(f, "cargo")?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

pub fn c_binding_step(cflags: &str) -> CargoStep {
    CargoStep::new(["test", "--features=__intern_c_bindings"])
        .with_env("SHISHUA_C_DIR", "shishua")
        .with_env("CFLAGS", cflags)
}

pub fn test_all_plan(
    arch: &str,
    has_c_bindings: bool,
    cpu_features: CpuFeatures,
) -> Vec<CargoStep> {
    let mut plan = vec![
        CargoStep::new(["test"]),
        CargoStep::new(["test", "--no-default-features"]),
        CargoStep::new(["test"]).with_env("RUSTFLAGS", "-C target-cpu=native"),
    ];

    if !has_c_bindings {
        return plan;
    }

    plan.push(c_binding_step("-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR"));

    match arch {
        "x86_64" | "amd64" => {
            plan.push(c_binding_step(
                "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2",
            ));
            if cpu_features.avx2 {
                plan.push(c_binding_step(
                    "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell",
                ));
            }
        },
        "x86" | "i386" | "i486" | "i586" | "i686" => {
            if cpu_features.sse2 {
                plan.push(c_binding_step(
                    "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2",
                ));
            }
            if cpu_features.avx2 {
                plan.push(c_binding_step(
                    "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell",
                ));
            }
        },
        "aarch64" | "arm64" => {
            plan.push(c_binding_step(
                "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_NEON",
            ));
        },
        _ => {},
    }

    plan
}

pub fn run(
    args: impl IntoIterator<Item = String>,
) -> Result<(), Box<dyn Error>> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("test-all") => run_test_all(),
        Some("-h") | Some("--help") | None => {
            print_usage();
            Ok(())
        },
        Some(command) => {
            Err(format!("unknown xtask command: {command}").into())
        },
    }
}

fn run_test_all() -> Result<(), Box<dyn Error>> {
    let workspace_root = workspace_root()?;
    let has_c_bindings = workspace_root.join("shishua/shishua.h").exists();
    let plan =
        test_all_plan(env::consts::ARCH, has_c_bindings, detect_cpu_features());

    for step in plan {
        step.run(&workspace_root)?;
    }

    Ok(())
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest directory has no parent".into())
}

fn detect_cpu_features() -> CpuFeatures {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        CpuFeatures {
            sse2: std::is_x86_feature_detected!("sse2"),
            avx2: std::is_x86_feature_detected!("avx2"),
        }
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        CpuFeatures::default()
    }
}

fn print_usage() {
    eprintln!("Usage: cargo run -p xtask -- test-all");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_without_c_bindings_runs_only_rust_checks() {
        let plan = test_all_plan("aarch64", false, CpuFeatures::default());

        assert_eq!(
            plan,
            vec![
                CargoStep::new(["test"]),
                CargoStep::new(["test", "--no-default-features"]),
                CargoStep::new(["test"])
                    .with_env("RUSTFLAGS", "-C target-cpu=native"),
            ],
        );
    }

    #[test]
    fn aarch64_c_binding_plan_runs_scalar_and_neon() {
        let plan = test_all_plan("aarch64", true, CpuFeatures::default());

        assert!(plan.contains(&c_binding_step(
            "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SCALAR"
        )));
        assert!(plan.contains(&c_binding_step(
            "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_NEON"
        )));
    }

    #[test]
    fn x86_64_c_binding_plan_runs_sse2_and_detected_avx2() {
        let plan = test_all_plan(
            "x86_64",
            true,
            CpuFeatures {
                sse2: true,
                avx2: true,
            },
        );

        assert!(plan.contains(&c_binding_step(
            "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_SSE2 -msse2 -mno-ssse3 -mno-avx -mno-avx2"
        )));
        assert!(plan.contains(&c_binding_step(
            "-O3 -DSHISHUA_TARGET=SHISHUA_TARGET_AVX2 -mavx2 -mtune=haswell"
        )));
    }
}
