#[cfg(target_feature = "neon")]
mod neon;
#[cfg(target_feature = "neon")]
pub use neon::Vector;
