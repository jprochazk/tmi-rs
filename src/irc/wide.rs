#![allow(dead_code)]

#[cfg(all(
  target_arch = "x86_64",
  any(target_feature = "sse2", target_feature = "avx2")
))]
pub(super) mod x86_64;

#[cfg(all(
  target_arch = "x86_64",
  any(target_feature = "sse2", target_feature = "avx2")
))]
pub use x86_64::find;

#[cfg(all(
  target_arch = "x86_64",
  not(any(target_feature = "sse2", target_feature = "avx2"))
))]
const _: () = {
  compile_error!("cannot use SIMD - please enable support for sse2 or avx2");
};

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
mod aarch64;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
pub use aarch64::find;

#[cfg(all(target_arch = "aarch64", not(target_feature = "neon")))]
const _: () = {
  compile_error!("cannot use SIMD - please enable supprot for neon");
};
