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
pub(super) use x86_64::Vector;

#[cfg(all(
  target_arch = "x86_64",
  not(any(target_feature = "sse2", target_feature = "avx2"))
))]
const _: () = {
  compile_error!(
    "cannot use SIMD - please enable support for sse2, avx2, or avx512 by compiling with target-cpu=native"
  );
};

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
pub(super) mod aarch64;

#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
pub(super) use aarch64::Vector;

#[cfg(all(target_arch = "aarch64", not(target_feature = "neon")))]
const _: () = {
  compile_error!(
    "cannot use SIMD - please enable support for neon by compiling with target-cpu=native"
  );
};
