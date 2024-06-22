cfg_if::cfg_if! {
    if #[cfg(target_feature = "neon")] {
        mod neon;
        pub(crate) use neon::Vector;
    } else {
        compile_error!(
            "enable the `neon` target features using `target-cpu=native`, or disable the `simd` feature"
        );
    }
}
