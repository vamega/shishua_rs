use crate::scalar_backend;

#[cfg(target_arch = "aarch64")]
use crate::neon_backend;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::{avx2_backend, sse2_backend, ssse3_backend};

pub const STATE_LANES: usize = 4;
pub const STATE_SIZE: usize = 4;
pub(crate) const BLOCK_BYTES: usize =
    STATE_LANES * STATE_SIZE * size_of::<u64>();

pub(crate) const PHI: [u64; 16] = [
    0x9E3779B97F4A7C15,
    0xF39CC0605CEDC834,
    0x1082276BF3A27251,
    0xF86C6A11D0C18E95,
    0x2767F0B153D27B7F,
    0x0347045B5BF1827F,
    0x01886F0928403002,
    0xC1D64BA40F335E36,
    0xF06AD7AE9717877E,
    0x85839D6EFFBD7DC6,
    0x64D325D1C5371682,
    0xCADD0CCCFDFFBBE1,
    0x626E33B8D04B4331,
    0xBBF73C790D94F79D,
    0x471C4AB3ED3D82A5,
    0xFEC507705E4AE6E5,
];

#[derive(Copy, Clone)]
enum StateImpl {
    Scalar(scalar_backend::State),
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Sse2(sse2_backend::State),
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Ssse3(ssse3_backend::State),
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    Avx2(avx2_backend::State),
    #[cfg(target_arch = "aarch64")]
    Neon(neon_backend::State),
}

/// The raw ShiShuA implementation.
///
/// `new` picks the fastest available backend at runtime where stable
/// `no_std` runtime detection is available. Use the explicit constructors to
/// force a specific backend for benchmarking.
#[derive(Copy, Clone)]
pub struct ShiShuAState {
    inner: StateImpl,
}

impl ShiShuAState {
    pub fn new(seed: [u64; STATE_LANES]) -> Self {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if Self::is_avx2_available() {
                return unsafe { Self::new_avx2(seed) };
            }
            if Self::is_ssse3_available() {
                return unsafe { Self::new_ssse3(seed) };
            }
            if Self::is_sse2_available() {
                return unsafe { Self::new_sse2(seed) };
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            unsafe { Self::new_neon(seed) }
        }

        #[cfg(not(target_arch = "aarch64"))]
        {
            Self::new_scalar(seed)
        }
    }

    pub fn new_scalar(seed: [u64; STATE_LANES]) -> Self {
        Self {
            inner: StateImpl::Scalar(scalar_backend::State::new(seed)),
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    /// Creates a state that always uses the SSE2 backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current CPU supports SSE2 before using
    /// the returned state. Prefer [`ShiShuAState::new`] for runtime dispatch.
    pub unsafe fn new_sse2(seed: [u64; STATE_LANES]) -> Self {
        Self {
            inner: StateImpl::Sse2(sse2_backend::State::new(seed)),
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    /// Creates a state that always uses the SSSE3 backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current CPU supports SSSE3 before using
    /// the returned state. Prefer [`ShiShuAState::new`] for runtime dispatch.
    pub unsafe fn new_ssse3(seed: [u64; STATE_LANES]) -> Self {
        Self {
            inner: StateImpl::Ssse3(ssse3_backend::State::new(seed)),
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    /// Creates a state that always uses the AVX2 backend.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the current CPU supports AVX2 and that the
    /// operating system has enabled AVX register state before using the
    /// returned state. Prefer [`ShiShuAState::new`] for runtime dispatch.
    pub unsafe fn new_avx2(seed: [u64; STATE_LANES]) -> Self {
        Self {
            inner: StateImpl::Avx2(avx2_backend::State::new(seed)),
        }
    }

    #[cfg(target_arch = "aarch64")]
    /// Creates a state that always uses the NEON backend.
    ///
    /// # Safety
    ///
    /// This constructor may only be used on targets where the generated binary
    /// can execute AArch64 NEON instructions. Prefer [`ShiShuAState::new`] for
    /// backend selection.
    pub unsafe fn new_neon(seed: [u64; STATE_LANES]) -> Self {
        Self {
            inner: StateImpl::Neon(neon_backend::State::new(seed)),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match &self.inner {
            StateImpl::Scalar(_) => "scalar",
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Sse2(_) => "sse2",
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Ssse3(_) => "ssse3",
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Avx2(_) => "avx2",
            #[cfg(target_arch = "aarch64")]
            StateImpl::Neon(_) => "neon",
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn is_sse2_available() -> bool {
        sse2_available()
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn is_ssse3_available() -> bool {
        ssse3_available()
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn is_avx2_available() -> bool {
        avx2_available()
    }

    #[cfg(target_arch = "aarch64")]
    pub fn is_neon_available() -> bool {
        true
    }

    pub fn round_unpack(&mut self) -> [u64; STATE_SIZE * STATE_LANES] {
        match &mut self.inner {
            StateImpl::Scalar(state) => state.round_unpack(),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Sse2(state) => unsafe { state.round_unpack() },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Ssse3(state) => unsafe { state.round_unpack() },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Avx2(state) => unsafe { state.round_unpack() },
            #[cfg(target_arch = "aarch64")]
            StateImpl::Neon(state) => unsafe { state.round_unpack() },
        }
    }

    #[cfg(feature = "rand")]
    pub(crate) fn generate_bytes(&mut self, output_slice: &mut [u8]) {
        match &mut self.inner {
            StateImpl::Scalar(state) => state.generate_bytes(output_slice),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Sse2(state) => unsafe {
                state.generate_bytes(output_slice)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Ssse3(state) => unsafe {
                state.generate_bytes(output_slice)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            StateImpl::Avx2(state) => unsafe {
                state.generate_bytes(output_slice)
            },
            #[cfg(target_arch = "aarch64")]
            StateImpl::Neon(state) => unsafe {
                state.generate_bytes(output_slice)
            },
        }
    }
}

#[cfg(target_arch = "x86")]
use core::arch::x86::{__cpuid, __cpuid_count, _xgetbv};
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, __cpuid_count, _xgetbv};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn sse2_available() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        true
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        (__cpuid(1).edx & (1 << 26)) != 0
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn ssse3_available() -> bool {
    sse2_available() && (__cpuid(1).ecx & (1 << 9)) != 0
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn avx2_available() -> bool {
    unsafe {
        if __cpuid(0).eax < 7 {
            return false;
        }

        let leaf1 = __cpuid(1);
        let avx = (leaf1.ecx & (1 << 28)) != 0;
        let osxsave = (leaf1.ecx & (1 << 27)) != 0;
        if !avx || !osxsave {
            return false;
        }

        let xcr0 = _xgetbv(0);
        if (xcr0 & 0b110) != 0b110 {
            return false;
        }

        (__cpuid_count(7, 0).ebx & (1 << 5)) != 0
    }
}

pub(crate) fn bytes_to_u64s(
    bytes: &[u8; BLOCK_BYTES],
) -> [u64; STATE_SIZE * STATE_LANES] {
    let mut output = [0u64; STATE_SIZE * STATE_LANES];
    for (index, chunk) in bytes.chunks_exact(size_of::<u64>()).enumerate() {
        let mut value = [0u8; size_of::<u64>()];
        value.copy_from_slice(chunk);
        output[index] = u64::from_le_bytes(value);
    }
    output
}
