#[cfg(feature = "client")]
#[macro_use]
extern crate tracing;

pub(crate) const fn assert_send<T: Send>() {}
macro_rules! assert_send {
  ($T:ty) => {
    const _: () = {
      let _ = $crate::assert_send::<$T>;
    };
  };
}

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "message-types")]
pub mod msg;
#[cfg(feature = "message-types")]
pub use msg::*;

pub mod irc;
pub use irc::*;

pub mod common;
pub use common::Span;

pub mod prelude {
  pub use crate::irc::{unescape, Command, IrcMessage, IrcMessageRef, Prefix, Tag, Whitelist};
}
