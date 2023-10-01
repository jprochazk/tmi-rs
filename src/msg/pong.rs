//! Sent by TMI as a response to a [`Ping`][Ping].
//!
//! If the [`Ping`][Ping] contained a [`Ping::nonce`][nonce],
//! the same nonce will be set to [`Pong::nonce`].
//!
//! [Ping]: crate::msg::ping::Ping
//! [nonce]: crate::msg::ping::Ping::nonce

use super::MessageParseError;
use crate::irc::{Command, IrcMessageRef};

/// Sent by TMI as a response to a [`Ping`][Ping].
///
/// If the [`Ping`][Ping] contained a [`Ping::nonce`][nonce],
/// the same nonce will be set to [`Pong::nonce`].
///
/// [Ping]: crate::msg::ping::Ping
/// [nonce]: crate::msg::ping::Ping::nonce
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pong<'src> {
  nonce: Option<&'src str>,
}

generate_getters! {
  <'src> for Pong<'src> as self {
    /// Unique string sent with this ping.
    nonce -> Option<&str>,
  }
}

impl<'src> Pong<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Pong {
      return None;
    }

    Some(Pong {
      nonce: message.text(),
    })
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
}
