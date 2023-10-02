#![doc = include_str!("../README.md")]

#[cfg(feature = "client")]
#[macro_use]
extern crate tracing;

pub(crate) const fn assert_sync<T: ?Sized + Sync>() {}
macro_rules! static_assert_sync {
  ($T:ty) => {
    const _: () = {
      let _ = $crate::assert_sync::<$T>;
    };
  };
}

pub(crate) const fn assert_send<T: ?Sized + Send>() {}
macro_rules! static_assert_send {
  ($T:ty) => {
    const _: () = {
      let _ = $crate::assert_send::<$T>;
    };
  };
}

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
pub use client::{Client, Credentials};

#[cfg(feature = "message-types")]
pub mod msg;
#[cfg(feature = "message-types")]
pub use msg::*;

pub mod irc;
pub use irc::*;

pub mod common;
pub use common::{Channel, ChannelRef};
