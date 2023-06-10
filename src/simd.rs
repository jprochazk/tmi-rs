#[cfg(all(target_arch = "x86_64", target_feature = "sse2"))]
pub mod x86_sse;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
pub mod arm_neon;
