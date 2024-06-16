// both `sse2` and `avx2` are available on practically every modern x64 CPU,
// but `avx2` has to be explicitly enabled for Rust using RUSTFLAGS:
//
// $ RUSTFLAGS="-C target-feature=+avx2" cargo build

// TODO: re-enable avx2 if there is a way to reduce the number of loads

// #[cfg(not(target_feature = "avx2"))]
mod sse2;
// #[cfg(not(target_feature = "avx2"))]
pub use sse2::find;

// #[cfg(target_feature = "avx2")]
// mod avx2;
// #[cfg(target_feature = "avx2")]
// pub use avx2::find;
