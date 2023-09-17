#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
#[macro_use]
extern crate tracing;

pub mod msg;

pub use msg::*;

pub mod prelude {
  pub use crate::msg::{
    parse, parse_with_whitelist, unescape, Command, Message, Prefix, Tag, Whitelist,
  };
}
