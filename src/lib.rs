#[cfg(feature = "client")]
#[macro_use]
extern crate tracing;

// #[cfg(feature = "client")]
// pub mod client;

pub mod msg;

pub use msg::*;

pub mod prelude {
  pub use crate::msg::{unescape, Command, Message, Prefix, Tag, Whitelist};
}
