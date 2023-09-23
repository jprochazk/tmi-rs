use crate::irc::{Command, IrcMessageRef};

/// Sent regularly by TMI to ensure clients are still live.
/// You must respond to TMI pings with a [`Pong`][Pong].
///
/// TMI will also respond with a pong if you send it a ping,
/// combined with the [`Ping::nonce`], this can be useful
/// to measure round-trip latency.
///
/// [Pong]: crate::msg::pong::Pong
#[derive(Clone, Debug)]
pub struct Ping<'src> {
  /// Unique string sent with this ping.
  pub nonce: Option<&'src str>,
}

impl<'src> super::FromIrc<'src> for Ping<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Ping {
      return None;
    }

    Some(Ping {
      nonce: message.text(),
    })
  }
}

impl<'src> From<Ping<'src>> for super::Message<'src> {
  fn from(msg: Ping<'src>) -> Self {
    super::Message::Ping(msg)
  }
}
