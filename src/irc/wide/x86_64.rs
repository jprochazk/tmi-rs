cfg_if::cfg_if! {
    if #[cfg(all(target_feature = "avx512f", target_feature = "avx512bw"))] {
        mod avx512;
        pub(crate) use avx512::Vector;
    } else if #[cfg(target_feature = "avx2")] {
        mod avx2;
        pub(crate) use avx2::Vector;
    } else if #[cfg(target_feature = "sse2")] {
        mod sse2;
        pub(crate) use sse2::Vector;
    } else {
        compile_error!(
            "enable the `sse2`, `avx2`, or `avx512` target features using `target-cpu=native`, or disable the `simd` feature"
        );
    }
}
