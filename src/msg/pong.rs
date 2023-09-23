use crate::irc::{Command, IrcMessageRef};

/// Sent by TMI as a response to a [`Ping`][Ping].
///
/// If the [`Ping`][Ping] contained a [`Ping::nonce`][nonce],
/// the same nonce will be set to [`Pong::nonce`].
///
/// [Ping]: crate::msg::ping::Ping
/// [nonce]: crate::msg::ping::Ping::nonce
#[derive(Clone, Debug)]
pub struct Pong<'src> {
  /// Unique string sent with this pong.
  pub nonce: Option<&'src str>,
}

impl<'src> super::FromIrc<'src> for Pong<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Pong {
      return None;
    }

    Some(Pong {
      nonce: message.text(),
    })
  }
}

impl<'src> From<Pong<'src>> for super::Message<'src> {
  fn from(msg: Pong<'src>) -> Self {
    super::Message::Pong(msg)
  }
}
