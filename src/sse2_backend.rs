#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use crate::core::{bytes_to_u64s, BLOCK_BYTES, PHI, STATE_LANES, STATE_SIZE};

#[derive(Copy, Clone)]
pub struct State {
    state: [__m128i; 8],
    output: [__m128i; 8],
    counter: [__m128i; 2],
}

impl State {
    #[target_feature(enable = "sse2")]
    pub unsafe fn new(seed: [u64; STATE_LANES]) -> Self {
        let zero = _mm_setzero_si128();
        let mut state = Self {
            state: [
                xor(set_epi64x(0, seed[0]), load_phi(0)),
                xor(set_epi64x(0, seed[1]), load_phi(2)),
                xor(set_epi64x(0, seed[2]), load_phi(4)),
                xor(set_epi64x(0, seed[3]), load_phi(6)),
                xor(set_epi64x(0, seed[2]), load_phi(8)),
                xor(set_epi64x(0, seed[3]), load_phi(10)),
                xor(set_epi64x(0, seed[0]), load_phi(12)),
                xor(set_epi64x(0, seed[1]), load_phi(14)),
            ],
            output: [zero; 8],
            counter: [zero; 2],
        };

        let mut buffer = [0u8; BLOCK_BYTES];
        for _ in 0..13 {
            state.generate_bytes_inner(&mut buffer);
            state.state[0] = state.output[6];
            state.state[1] = state.output[7];
            state.state[2] = state.output[4];
            state.state[3] = state.output[5];
            state.state[4] = state.output[2];
            state.state[5] = state.output[3];
            state.state[6] = state.output[0];
            state.state[7] = state.output[1];
        }

        state
    }

    #[target_feature(enable = "sse2")]
    pub unsafe fn round_unpack(&mut self) -> [u64; STATE_SIZE * STATE_LANES] {
        let mut bytes = [0u8; BLOCK_BYTES];
        self.generate_bytes_inner(&mut bytes);
        bytes_to_u64s(&bytes)
    }

    #[cfg(feature = "rand")]
    #[target_feature(enable = "sse2")]
    pub unsafe fn generate_bytes(&mut self, output_slice: &mut [u8]) {
        self.generate_bytes_inner(output_slice);
    }

    #[target_feature(enable = "sse2")]
    unsafe fn generate_bytes_inner(&mut self, output_slice: &mut [u8]) {
        assert_eq!(output_slice.len() % BLOCK_BYTES, 0);

        let mut state = self.state;
        let mut output = self.output;
        let mut counter_lo = self.counter[0];
        let mut counter_hi = self.counter[1];
        let increment_lo = _mm_set_epi32(0, 5, 0, 7);
        let increment_hi = _mm_set_epi32(0, 1, 0, 3);

        for output_chunk in output_slice.chunks_exact_mut(BLOCK_BYTES) {
            for (index, value) in output.iter().enumerate() {
                _mm_storeu_si128(
                    output_chunk[index * 16..].as_mut_ptr().cast(),
                    *value,
                );
            }

            for j in 0..2 {
                let mut s_lo = state[4 * j];
                let mut s_hi = state[4 * j + 1];
                let u0_lo = _mm_srli_epi64::<1>(s_lo);
                let u0_hi = _mm_srli_epi64::<1>(s_hi);
                let t0_lo = _mm_or_si128(
                    _mm_slli_si128::<12>(s_lo),
                    _mm_srli_si128::<4>(s_hi),
                );
                let t0_hi = _mm_or_si128(
                    _mm_slli_si128::<12>(s_hi),
                    _mm_srli_si128::<4>(s_lo),
                );
                state[4 * j] = _mm_add_epi64(t0_lo, u0_lo);
                state[4 * j + 1] = _mm_add_epi64(t0_hi, u0_hi);

                s_lo = _mm_add_epi64(state[4 * j + 2], counter_lo);
                s_hi = _mm_add_epi64(state[4 * j + 3], counter_hi);
                let u1_lo = _mm_srli_epi64::<3>(s_lo);
                let u1_hi = _mm_srli_epi64::<3>(s_hi);
                let t1_lo = _mm_or_si128(
                    _mm_slli_si128::<4>(s_hi),
                    _mm_srli_si128::<12>(s_lo),
                );
                let t1_hi = _mm_or_si128(
                    _mm_slli_si128::<4>(s_lo),
                    _mm_srli_si128::<12>(s_hi),
                );
                state[4 * j + 2] = _mm_add_epi64(t1_lo, u1_lo);
                state[4 * j + 3] = _mm_add_epi64(t1_hi, u1_hi);

                output[2 * j] = _mm_xor_si128(u0_lo, t1_lo);
                output[2 * j + 1] = _mm_xor_si128(u0_hi, t1_hi);
            }

            output[4] = _mm_xor_si128(state[0], state[6]);
            output[5] = _mm_xor_si128(state[1], state[7]);
            output[6] = _mm_xor_si128(state[4], state[2]);
            output[7] = _mm_xor_si128(state[5], state[3]);

            counter_lo = _mm_add_epi64(counter_lo, increment_lo);
            counter_hi = _mm_add_epi64(counter_hi, increment_hi);
        }

        self.state = state;
        self.output = output;
        self.counter[0] = counter_lo;
        self.counter[1] = counter_hi;
    }
}

#[target_feature(enable = "sse2")]
unsafe fn set_epi64x(high: u64, low: u64) -> __m128i {
    _mm_set_epi32(
        (high >> 32) as i32,
        high as i32,
        (low >> 32) as i32,
        low as i32,
    )
}

#[target_feature(enable = "sse2")]
unsafe fn load_phi(index: usize) -> __m128i {
    set_epi64x(PHI[index + 1], PHI[index])
}

#[target_feature(enable = "sse2")]
unsafe fn xor(lhs: __m128i, rhs: __m128i) -> __m128i {
    _mm_xor_si128(lhs, rhs)
}
