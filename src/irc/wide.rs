cfg_if::cfg_if! {
  if #[cfg(all(
    target_arch = "x86_64",
    any(
      target_feature = "sse2",
      target_feature = "avx2",
      all(target_feature = "avx512f", target_feature = "avx512bw")
    )
  ))] {
    pub(super) mod x86_64;
    pub(super) use x86_64::Vector;
  } else if #[cfg(all(
    target_arch = "aarch64",
    target_feature = "neon"
  ))] {
    pub(super) mod aarch64;
    pub(super) use aarch64::Vector;
  } else {
    compile_error!("unsupported target architecture - please disable the `simd` feature");
  }
}
