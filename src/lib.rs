#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::missing_doc_code_examples)]

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
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "message-types")]
pub mod msg;
#[cfg(feature = "message-types")]
pub use msg::*;

pub mod irc;
pub use irc::*;

pub mod common;
