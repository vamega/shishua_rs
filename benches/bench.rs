use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use rand_core::RngCore;
use shishua::{ShiShuARng, ShiShuAState};

#[cfg(feature = "__intern_c_bindings")]
extern "C" {
    fn shishua_bindings_init(seed: *const u64) -> *mut ();
    fn shishua_bindings_destroy(state: *mut ());
    fn shishua_bindings_generate(state: *mut (), buffer: *mut u8, size: usize);
}

pub fn benchmark_shisuha(c: &mut Criterion) {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;

    let seed = [0x1, 0x2, 0x3, 0x4];
    #[cfg(feature = "__intern_c_bindings")]
    let native_rng = unsafe { shishua_bindings_init(seed.as_ptr()) };

    let mut group = c.benchmark_group("throughput");

    for size in [512, KB, MB] {
        assert_eq!(size % 512, 0);

        group.throughput(Throughput::Bytes(size as u64));

        let mut runtime = ShiShuARng::new(seed);
        bench_rng(
            &mut group,
            format!("shishua_rs_runtime_{}", runtime.backend_name()),
            size,
            &mut runtime,
        );

        let mut scalar = ShiShuARng::new_scalar(seed);
        bench_rng(&mut group, "shishua_rs_scalar", size, &mut scalar);

        #[cfg(target_arch = "aarch64")]
        {
            let mut neon = unsafe { ShiShuARng::new_neon(seed) };
            bench_rng(&mut group, "shishua_rs_neon", size, &mut neon);
        }

        #[cfg(all(
            any(target_arch = "x86", target_arch = "x86_64"),
            not(miri)
        ))]
        {
            if ShiShuAState::is_sse2_available() {
                let mut sse2 = unsafe { ShiShuARng::new_sse2(seed) };
                bench_rng(&mut group, "shishua_rs_sse2", size, &mut sse2);
            }

            if ShiShuAState::is_avx2_available() {
                let mut avx2 = unsafe { ShiShuARng::new_avx2(seed) };
                bench_rng(&mut group, "shishua_rs_avx2", size, &mut avx2);
            }
        }

        #[cfg(feature = "__intern_c_bindings")]
        let mut buffer = vec![0; size];
        #[cfg(feature = "__intern_c_bindings")]
        group.bench_function(BenchmarkId::new("shishua_c", size), |b| {
            b.iter(|| unsafe {
                shishua_bindings_generate(native_rng, buffer.as_mut_ptr(), size)
            });
        });
    }

    #[cfg(feature = "__intern_c_bindings")]
    unsafe {
        shishua_bindings_destroy(native_rng)
    };
    group.finish();
}

fn bench_rng(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: impl Into<String>,
    size: usize,
    rng: &mut ShiShuARng,
) {
    let mut buffer = vec![0; size];
    group.bench_function(BenchmarkId::new(name.into(), size), |b| {
        b.iter(|| rng.fill_bytes(buffer.as_mut_slice()))
    });
}

criterion_group!(benches, benchmark_shisuha);
criterion_main!(benches);
