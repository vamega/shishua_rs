use crate::core::{BLOCK_BYTES, PHI, STATE_LANES, STATE_SIZE};

#[derive(Copy, Clone)]
pub struct State {
    state: [u64; STATE_SIZE * STATE_LANES],
    output: [u64; STATE_SIZE * STATE_LANES],
    counter: [u64; STATE_LANES],
}

impl State {
    pub fn new(seed: [u64; STATE_LANES]) -> Self {
        let mut state = Self {
            state: PHI,
            output: [0; STATE_SIZE * STATE_LANES],
            counter: [0; STATE_LANES],
        };

        for i in 0..STATE_LANES {
            state.state[i * 2] ^= seed[i];
            state.state[i * 2 + 8] ^= seed[(i + 2) % STATE_LANES];
        }

        for _ in 0..13 {
            state.round(None);
            state.state[0] = state.output[12];
            state.state[1] = state.output[13];
            state.state[2] = state.output[14];
            state.state[3] = state.output[15];
            state.state[4] = state.output[8];
            state.state[5] = state.output[9];
            state.state[6] = state.output[10];
            state.state[7] = state.output[11];
            state.state[8] = state.output[4];
            state.state[9] = state.output[5];
            state.state[10] = state.output[6];
            state.state[11] = state.output[7];
            state.state[12] = state.output[0];
            state.state[13] = state.output[1];
            state.state[14] = state.output[2];
            state.state[15] = state.output[3];
        }

        state
    }

    pub fn round_unpack(&mut self) -> [u64; STATE_SIZE * STATE_LANES] {
        let output = self.output;
        self.round(None);
        output
    }

    #[cfg(feature = "rand")]
    pub fn generate_bytes(&mut self, output_slice: &mut [u8]) {
        assert_eq!(output_slice.len() % BLOCK_BYTES, 0);

        for output_chunk in output_slice.chunks_exact_mut(BLOCK_BYTES) {
            self.round(Some(output_chunk));
        }
    }

    fn round(&mut self, output_slice: Option<&mut [u8]>) {
        if let Some(output_slice) = output_slice {
            debug_assert_eq!(output_slice.len(), BLOCK_BYTES);
            for (output_chunk, value) in output_slice
                .chunks_exact_mut(size_of::<u64>())
                .zip(self.output)
            {
                output_chunk.copy_from_slice(&value.to_le_bytes());
            }
        }

        const SHUF_OFFSETS: [usize; 16] =
            [2, 3, 0, 1, 5, 6, 7, 4, 3, 0, 1, 2, 6, 7, 4, 5];

        for j in 0..2 {
            let state_offset = j * 8;
            let output_offset = j * 4;
            let mut temp = [0u64; 8];

            for k in 0..4 {
                self.state[state_offset + k + 4] = self.state
                    [state_offset + k + 4]
                    .wrapping_add(self.counter[k]);
            }

            for k in 0..8 {
                temp[k] = (self.state[state_offset + SHUF_OFFSETS[k]] >> 32)
                    | (self.state[state_offset + SHUF_OFFSETS[k + 8]] << 32);
            }

            for k in 0..4 {
                let u_lo = self.state[state_offset + k] >> 1;
                let u_hi = self.state[state_offset + k + 4] >> 3;

                self.state[state_offset + k] = u_lo.wrapping_add(temp[k]);
                self.state[state_offset + k + 4] =
                    u_hi.wrapping_add(temp[k + 4]);
                self.output[output_offset + k] = u_lo ^ temp[k + 4];
            }
        }

        for j in 0..4 {
            self.output[j + 8] = self.state[j] ^ self.state[j + 12];
            self.output[j + 12] = self.state[j + 8] ^ self.state[j + 4];
            self.counter[j] = self.counter[j].wrapping_add(7 - (j as u64 * 2));
        }
    }
}
