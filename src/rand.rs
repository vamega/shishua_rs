use crate::{
    core::{STATE_LANES, STATE_SIZE},
    ShiShuAState,
};
use rand_core::{RngCore, SeedableRng};

const STATE_WRAPPER_BUFFER_SIZE: usize =
    STATE_LANES * STATE_SIZE * size_of::<u64>();

/// A rand compatible wrapper around the raw ShiShuAState.
///
/// An internal buffer is used to split up big chunks of randomness into the requested size.
pub struct ShiShuARng {
    state: ShiShuAState,
    buffer: [u8; STATE_WRAPPER_BUFFER_SIZE],
    buffer_index: usize,
}

impl ShiShuARng {
    pub fn new(seed: [u64; STATE_LANES]) -> Self {
        Self::from_state(ShiShuAState::new(seed))
    }

    pub fn from_state(state: ShiShuAState) -> Self {
        ShiShuARng {
            state,
            buffer: [0; STATE_WRAPPER_BUFFER_SIZE],
            buffer_index: STATE_WRAPPER_BUFFER_SIZE,
        }
    }

    pub fn new_scalar(seed: [u64; STATE_LANES]) -> Self {
        Self::from_state(ShiShuAState::new_scalar(seed))
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    /// Creates an RNG that always uses the SSE2 backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current CPU supports SSE2 before using
    /// the returned RNG. Prefer [`ShiShuARng::new`] for runtime dispatch.
    pub unsafe fn new_sse2(seed: [u64; STATE_LANES]) -> Self {
        Self::from_state(ShiShuAState::new_sse2(seed))
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    /// Creates an RNG that always uses the SSSE3 backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current CPU supports SSSE3 before using
    /// the returned RNG. Prefer [`ShiShuARng::new`] for runtime dispatch.
    pub unsafe fn new_ssse3(seed: [u64; STATE_LANES]) -> Self {
        Self::from_state(ShiShuAState::new_ssse3(seed))
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    /// Creates an RNG that always uses the AVX2 backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current CPU supports AVX2 and that the
    /// operating system has enabled AVX register state before using the
    /// returned RNG. Prefer [`ShiShuARng::new`] for runtime dispatch.
    pub unsafe fn new_avx2(seed: [u64; STATE_LANES]) -> Self {
        Self::from_state(ShiShuAState::new_avx2(seed))
    }

    #[cfg(target_arch = "aarch64")]
    /// Creates an RNG that always uses the NEON backend.
    ///
    /// # Safety
    ///
    /// This constructor may only be used on targets where the generated binary
    /// can execute AArch64 NEON instructions. Prefer [`ShiShuARng::new`] for
    /// backend selection.
    pub unsafe fn new_neon(seed: [u64; STATE_LANES]) -> Self {
        Self::from_state(ShiShuAState::new_neon(seed))
    }

    pub fn backend_name(&self) -> &'static str {
        self.state.backend_name()
    }

    #[inline(always)]
    pub fn get_byte(&mut self) -> u8 {
        if self.buffer_index >= STATE_WRAPPER_BUFFER_SIZE {
            self.buffer_index = 0;
            self.state.generate_bytes(&mut self.buffer);
        }

        let index = self.buffer_index;
        self.buffer_index += 1;

        self.buffer[index]
    }
}

impl RngCore for ShiShuARng {
    fn next_u32(&mut self) -> u32 {
        let mut buffer = [0u8; size_of::<u32>()];
        self.fill_bytes(&mut buffer);
        u32::from_le_bytes(buffer)
    }

    fn next_u64(&mut self) -> u64 {
        let mut buffer = [0u8; size_of::<u64>()];
        self.fill_bytes(&mut buffer);
        u64::from_le_bytes(buffer)
    }

    fn fill_bytes(&mut self, mut dest: &mut [u8]) {
        let buffered =
            (STATE_WRAPPER_BUFFER_SIZE - self.buffer_index).min(dest.len());
        if buffered > 0 {
            dest[..buffered].copy_from_slice(
                &self.buffer[self.buffer_index..self.buffer_index + buffered],
            );
            self.buffer_index += buffered;
            dest = &mut dest[buffered..];
        }

        let block_bytes =
            dest.len() / STATE_WRAPPER_BUFFER_SIZE * STATE_WRAPPER_BUFFER_SIZE;
        if block_bytes > 0 {
            let (blocks, tail) = dest.split_at_mut(block_bytes);
            self.state.generate_bytes(blocks);
            dest = tail;
        }

        for byte in dest.iter_mut() {
            *byte = self.get_byte();
        }
    }
}

impl SeedableRng for ShiShuARng {
    type Seed = [u8; STATE_LANES * size_of::<u64>()];

    fn from_seed(seed: Self::Seed) -> Self {
        let mut words = [0u64; STATE_LANES];
        for (word, chunk) in
            words.iter_mut().zip(seed.chunks_exact(size_of::<u64>()))
        {
            let mut bytes = [0u8; size_of::<u64>()];
            bytes.copy_from_slice(chunk);
            *word = u64::from_le_bytes(bytes);
        }
        Self::new(words)
    }
}
