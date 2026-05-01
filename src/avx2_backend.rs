#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use crate::core::{bytes_to_u64s, BLOCK_BYTES, PHI, STATE_LANES, STATE_SIZE};

#[derive(Copy, Clone)]
pub struct State {
    state: [__m256i; STATE_SIZE],
    output: [__m256i; STATE_SIZE],
    counter: __m256i,
}

impl State {
    #[target_feature(enable = "avx2")]
    pub unsafe fn new(seed: [u64; STATE_LANES]) -> Self {
        let zero = _mm256_setzero_si256();
        let mut state = Self {
            state: [
                _mm256_set_epi64x(
                    PHI[3] as i64,
                    (PHI[2] ^ seed[1]) as i64,
                    PHI[1] as i64,
                    (PHI[0] ^ seed[0]) as i64,
                ),
                _mm256_set_epi64x(
                    PHI[7] as i64,
                    (PHI[6] ^ seed[3]) as i64,
                    PHI[5] as i64,
                    (PHI[4] ^ seed[2]) as i64,
                ),
                _mm256_set_epi64x(
                    PHI[11] as i64,
                    (PHI[10] ^ seed[3]) as i64,
                    PHI[9] as i64,
                    (PHI[8] ^ seed[2]) as i64,
                ),
                _mm256_set_epi64x(
                    PHI[15] as i64,
                    (PHI[14] ^ seed[1]) as i64,
                    PHI[13] as i64,
                    (PHI[12] ^ seed[0]) as i64,
                ),
            ],
            output: [zero; STATE_SIZE],
            counter: zero,
        };

        let mut buffer = [0u8; BLOCK_BYTES];
        for _ in 0..13 {
            state.generate_bytes_inner(&mut buffer);
            state.state[0] = state.output[3];
            state.state[1] = state.output[2];
            state.state[2] = state.output[1];
            state.state[3] = state.output[0];
        }

        state
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn round_unpack(&mut self) -> [u64; STATE_SIZE * STATE_LANES] {
        let mut bytes = [0u8; BLOCK_BYTES];
        self.generate_bytes_inner(&mut bytes);
        bytes_to_u64s(&bytes)
    }

    #[cfg(feature = "rand")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn generate_bytes(&mut self, output_slice: &mut [u8]) {
        self.generate_bytes_inner(output_slice);
    }

    #[target_feature(enable = "avx2")]
    unsafe fn generate_bytes_inner(&mut self, output_slice: &mut [u8]) {
        assert_eq!(output_slice.len() % BLOCK_BYTES, 0);

        let mut o0 = self.output[0];
        let mut o1 = self.output[1];
        let mut o2 = self.output[2];
        let mut o3 = self.output[3];
        let mut s0 = self.state[0];
        let mut s1 = self.state[1];
        let mut s2 = self.state[2];
        let mut s3 = self.state[3];
        let mut counter = self.counter;
        let shuffle0 = _mm256_set_epi32(4, 3, 2, 1, 0, 7, 6, 5);
        let shuffle1 = _mm256_set_epi32(2, 1, 0, 7, 6, 5, 4, 3);
        let increment = _mm256_set_epi64x(1, 3, 5, 7);

        for output_chunk in output_slice.chunks_exact_mut(BLOCK_BYTES) {
            _mm256_storeu_si256(output_chunk.as_mut_ptr().cast(), o0);
            _mm256_storeu_si256(output_chunk[32..].as_mut_ptr().cast(), o1);
            _mm256_storeu_si256(output_chunk[64..].as_mut_ptr().cast(), o2);
            _mm256_storeu_si256(output_chunk[96..].as_mut_ptr().cast(), o3);

            s1 = _mm256_add_epi64(s1, counter);
            s3 = _mm256_add_epi64(s3, counter);
            counter = _mm256_add_epi64(counter, increment);

            let u0 = _mm256_srli_epi64::<1>(s0);
            let u1 = _mm256_srli_epi64::<3>(s1);
            let u2 = _mm256_srli_epi64::<1>(s2);
            let u3 = _mm256_srli_epi64::<3>(s3);

            let t0 = _mm256_permutevar8x32_epi32(s0, shuffle0);
            let t1 = _mm256_permutevar8x32_epi32(s1, shuffle1);
            let t2 = _mm256_permutevar8x32_epi32(s2, shuffle0);
            let t3 = _mm256_permutevar8x32_epi32(s3, shuffle1);

            s0 = _mm256_add_epi64(t0, u0);
            s1 = _mm256_add_epi64(t1, u1);
            s2 = _mm256_add_epi64(t2, u2);
            s3 = _mm256_add_epi64(t3, u3);

            o0 = _mm256_xor_si256(u0, t1);
            o1 = _mm256_xor_si256(u2, t3);
            o2 = _mm256_xor_si256(s0, s3);
            o3 = _mm256_xor_si256(s2, s1);
        }

        self.output[0] = o0;
        self.output[1] = o1;
        self.output[2] = o2;
        self.output[3] = o3;
        self.state[0] = s0;
        self.state[1] = s1;
        self.state[2] = s2;
        self.state[3] = s3;
        self.counter = counter;
    }
}
