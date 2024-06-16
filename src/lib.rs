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

use std::borrow::Cow;

/// Checks if `value` needs to be unescaped by looking for escaped characters.
///
/// If it must be unescaped, then it must reallocate and will return an owned string.
/// Otherwise, it returns a borrow of the original `value`.
pub fn maybe_unescape<'a>(value: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
  let mut value: Cow<'_, str> = value.into();
  for i in 0..value.len() {
    if value.as_bytes()[i] == b'\\' {
      value = Cow::Owned(actually_unescape(&value, i));
      break;
    }
  }
  value
}

#[inline]
fn actually_unescape(input: &str, start: usize) -> String {
  let mut out = String::with_capacity(input.len());
  out.push_str(&input[..start]);

  let mut escape = false;
  for char in input[start..].chars() {
    match char {
      '\\' if escape => {
        out.push('\\');
        escape = false;
      }
      '\\' => escape = true,
      ':' if escape => {
        out.push(';');
        escape = false;
      }
      's' if escape => {
        out.push(' ');
        escape = false;
      }
      'r' if escape => {
        out.push('\r');
        escape = false;
      }
      'n' if escape => {
        out.push('\n');
        escape = false;
      }
      '⸝' => out.push(','),
      c => out.push(c),
    }
  }

  out
}
