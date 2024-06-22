// both `sse2` and `avx2` are available on practically every modern x64 CPU,
// but `avx2` has to be explicitly enabled for Rust using RUSTFLAGS:
//
// $ RUSTFLAGS="-C target-feature=+avx2" cargo build

// TODO: re-enable avx2 if there is a way to reduce the number of loads

#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
pub(crate) mod sse2;
#[cfg(not(any(target_feature = "avx2", target_feature = "avx512f")))]
pub(crate) use sse2::Vector;

// #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
mod avx2;
// #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
pub(crate) use avx2::Vector;

// #[cfg(target_feature = "avx512f")]
// mod avx512;
// #[cfg(target_feature = "avx512f")]
// pub(crate) use avx512::Vector;
