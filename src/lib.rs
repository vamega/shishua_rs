#![no_std]

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod avx2_backend;
pub(crate) mod core;
#[cfg(target_arch = "aarch64")]
mod neon_backend;
#[cfg(feature = "rand")]
pub(crate) mod rand;
mod scalar_backend;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod sse2_backend;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod ssse3_backend;

pub use crate::core::ShiShuAState;
#[cfg(feature = "rand")]
pub use crate::rand::ShiShuARng;
