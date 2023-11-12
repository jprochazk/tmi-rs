//! Sent regularly by TMI to ensure clients are still live.
//! You must respond to TMI pings with a [`Pong`][Pong].
//!
//! TMI will also respond with a pong if you send it a ping,
//! combined with the [`Ping::nonce`], this can be useful
//! to measure round-trip latency.
//!
//! [Pong]: crate::msg::pong::Pong

use super::MessageParseError;
use crate::irc::{Command, IrcMessageRef};
use std::borrow::Cow;

/// Sent regularly by TMI to ensure clients are still live.
/// You must respond to TMI pings with a [`Pong`][Pong].
///
/// TMI will also respond with a pong if you send it a ping,
/// combined with the [`Ping::nonce`], this can be useful
/// to measure round-trip latency.
///
/// [Pong]: crate::msg::pong::Pong
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ping<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  nonce: Option<Cow<'src, str>>,
}

generate_getters! {
  <'src> for Ping<'src> as self {
    /// Unique string sent with this ping.
    nonce -> Option<&str> = self.nonce.as_deref(),
  }
}

impl<'src> Ping<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Ping {
      return None;
    }

    Some(Ping {
      nonce: message.text().map(Cow::Borrowed),
    })
  }
}

impl<'src> super::FromIrc<'src> for Ping<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<Ping<'src>> for super::Message<'src> {
  fn from(msg: Ping<'src>) -> Self {
    super::Message::Ping(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_ping() {
    assert_irc_snapshot!(Ping, ":tmi.twitch.tv PING");
  }

  #[test]
  fn parse_ping_nonce() {
    assert_irc_snapshot!(Ping, ":tmi.twitch.tv PING :nonce");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_ping() {
    assert_irc_roundtrip!(Ping, ":tmi.twitch.tv PING");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_ping_nonce() {
    assert_irc_roundtrip!(Ping, ":tmi.twitch.tv PING :nonce");
  }
}
