//! Sent by TMI as a response to a [`Ping`][Ping].
//!
//! If the [`Ping`][Ping] contained a [`Ping::nonce`][nonce],
//! the same nonce will be set to [`Pong::nonce`].
//!
//! [Ping]: crate::msg::ping::Ping
//! [nonce]: crate::msg::ping::Ping::nonce

use super::{maybe_clone, MessageParseError};
use crate::irc::{Command, IrcMessageRef};
use std::borrow::Cow;

/// Sent by TMI as a response to a [`Ping`][Ping].
///
/// If the [`Ping`][Ping] contained a [`Ping::nonce`][nonce],
/// the same nonce will be set to [`Pong::nonce`].
///
/// [Ping]: crate::msg::ping::Ping
/// [nonce]: crate::msg::ping::Ping::nonce
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pong<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  nonce: Option<Cow<'src, str>>,
}

generate_getters! {
  <'src> for Pong<'src> as self {
    /// Unique string sent with this ping.
    nonce -> Option<&str> = self.nonce.as_deref(),
  }
}

impl<'src> Pong<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Pong {
      return None;
    }

    Some(Pong {
      nonce: message.text().map(Cow::Borrowed),
    })
  }

  /// Clone data to give the value a `'static` lifetime.
  pub fn into_owned(self) -> Pong<'static> {
    Pong {
      nonce: self.nonce.map(maybe_clone),
    }
  }
}

impl<'src> super::FromIrc<'src> for Pong<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<Pong<'src>> for super::Message<'src> {
  fn from(msg: Pong<'src>) -> Self {
    super::Message::Pong(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_ping() {
    assert_irc_snapshot!(Pong, ":tmi.twitch.tv PONG");
  }

  #[test]
  fn parse_ping_nonce() {
    assert_irc_snapshot!(Pong, ":tmi.twitch.tv PONG :nonce");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_ping() {
    assert_irc_roundtrip!(Pong, ":tmi.twitch.tv PONG");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_ping_nonce() {
    assert_irc_roundtrip!(Pong, ":tmi.twitch.tv PONG :nonce");
  }
}
