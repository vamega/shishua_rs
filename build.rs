fn main() {
    #[cfg(feature = "__intern_c_bindings")]
    {
        use std::{env, path::PathBuf};

        let shishua_dir = PathBuf::from(
            env::var("SHISHUA_C_DIR").unwrap_or_else(|_| "shishua".to_owned()),
        );
        let shishua_header = shishua_dir.join("shishua.h");

        if !shishua_header.exists() {
            panic!(
                "C bindings require shishua.h. Clone https://github.com/espadrine/shishua into ./shishua or set SHISHUA_C_DIR."
            );
        }

        println!("cargo:rerun-if-env-changed=SHISHUA_C_DIR");
        println!("cargo:rerun-if-changed=test_c/shishua_bindings.c");
        for header in [
            "shishua.h",
            "shishua-avx2.h",
            "shishua-sse2.h",
            "shishua-neon.h",
        ] {
            let header = shishua_dir.join(header);
            if header.exists() {
                println!("cargo:rerun-if-changed={}", header.display());
            }
        }

        cc::Build::new()
            .include(shishua_dir)
            .file("test_c/shishua_bindings.c")
            .compile("shishua_bindings");
    }
}
