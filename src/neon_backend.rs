use core::arch::aarch64::*;

use crate::core::{bytes_to_u64s, BLOCK_BYTES, PHI, STATE_LANES, STATE_SIZE};

#[derive(Copy, Clone)]
pub struct State {
    state: [uint64x2_t; 8],
    output: [uint64x2_t; 8],
    counter: [uint64x2_t; 2],
}

impl State {
    pub unsafe fn new(seed: [u64; STATE_LANES]) -> Self {
        let zero = vdupq_n_u64(0);
        let mut state = Self {
            state: [
                veorq_u64(set_u64x2(seed[0], 0), load_phi(0)),
                veorq_u64(set_u64x2(seed[1], 0), load_phi(2)),
                veorq_u64(set_u64x2(seed[2], 0), load_phi(4)),
                veorq_u64(set_u64x2(seed[3], 0), load_phi(6)),
                veorq_u64(set_u64x2(seed[2], 0), load_phi(8)),
                veorq_u64(set_u64x2(seed[3], 0), load_phi(10)),
                veorq_u64(set_u64x2(seed[0], 0), load_phi(12)),
                veorq_u64(set_u64x2(seed[1], 0), load_phi(14)),
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

    pub unsafe fn round_unpack(&mut self) -> [u64; STATE_SIZE * STATE_LANES] {
        let mut bytes = [0u8; BLOCK_BYTES];
        self.generate_bytes_inner(&mut bytes);
        bytes_to_u64s(&bytes)
    }

    #[cfg(feature = "rand")]
    pub unsafe fn generate_bytes(&mut self, output_slice: &mut [u8]) {
        self.generate_bytes_inner(output_slice);
    }

    unsafe fn generate_bytes_inner(&mut self, output_slice: &mut [u8]) {
        assert_eq!(output_slice.len() % BLOCK_BYTES, 0);

        let mut state = self.state;
        let mut output = self.output;
        let mut counter_lo = self.counter[0];
        let mut counter_hi = self.counter[1];
        let increment_lo = set_u64x2(7, 5);
        let increment_hi = set_u64x2(3, 1);

        for output_chunk in output_slice.chunks_exact_mut(BLOCK_BYTES) {
            for (index, value) in output.iter().enumerate() {
                vst1q_u8(
                    output_chunk[index * 16..].as_mut_ptr(),
                    vreinterpretq_u8_u64(*value),
                );
            }

            for j in 0..2 {
                let s0_lo = state[4 * j];
                let s0_hi = state[4 * j + 1];
                let s1_lo = vaddq_u64(state[4 * j + 2], counter_lo);
                let s1_hi = vaddq_u64(state[4 * j + 3], counter_hi);

                let t0_lo = vext_u64x2(s0_hi, s0_lo, 4);
                let t0_hi = vext_u64x2(s0_lo, s0_hi, 4);
                let t1_lo = vext_u64x2(s1_lo, s1_hi, 12);
                let t1_hi = vext_u64x2(s1_hi, s1_lo, 12);

                let u_lo = vshrq_n_u64::<1>(s0_lo);
                let u_hi = vshrq_n_u64::<1>(s0_hi);

                state[4 * j] = vaddq_u64(t0_lo, u_lo);
                state[4 * j + 1] = vaddq_u64(t0_hi, u_hi);
                state[4 * j + 2] = vaddq_u64(t1_lo, vshrq_n_u64::<3>(s1_lo));
                state[4 * j + 3] = vaddq_u64(t1_hi, vshrq_n_u64::<3>(s1_hi));

                output[2 * j] = veorq_u64(u_lo, t1_lo);
                output[2 * j + 1] = veorq_u64(u_hi, t1_hi);
            }

            output[4] = veorq_u64(state[0], state[6]);
            output[5] = veorq_u64(state[1], state[7]);
            output[6] = veorq_u64(state[2], state[4]);
            output[7] = veorq_u64(state[3], state[5]);

            counter_lo = vaddq_u64(counter_lo, increment_lo);
            counter_hi = vaddq_u64(counter_hi, increment_hi);
        }

        self.state = state;
        self.output = output;
        self.counter[0] = counter_lo;
        self.counter[1] = counter_hi;
    }
}

unsafe fn set_u64x2(low: u64, high: u64) -> uint64x2_t {
    vcombine_u64(vdup_n_u64(low), vdup_n_u64(high))
}

unsafe fn load_phi(index: usize) -> uint64x2_t {
    set_u64x2(PHI[index], PHI[index + 1])
}

unsafe fn vext_u64x2(
    rn: uint64x2_t,
    rm: uint64x2_t,
    amount: i32,
) -> uint64x2_t {
    match amount {
        4 => vreinterpretq_u64_u8(vextq_u8::<4>(
            vreinterpretq_u8_u64(rn),
            vreinterpretq_u8_u64(rm),
        )),
        12 => vreinterpretq_u64_u8(vextq_u8::<12>(
            vreinterpretq_u8_u64(rn),
            vreinterpretq_u8_u64(rm),
        )),
        _ => unreachable!(),
    }
}
