use crate::irc::{Command, IrcMessageRef};

/// Sent regularly by TMI to ensure clients are still live.
/// You must respond to TMI pings with a [`Pong`][Pong].
///
/// TMI will also respond with a pong if you send it a ping,
/// combined with the [`Ping::nonce`], this can be useful
/// to measure round-trip latency.
///
/// [Pong]: crate::msg::pong::Pong
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ping<'src> {
  nonce: Option<&'src str>,
}

generate_getters! {
  <'src> for Ping<'src> as self {
    /// Unique string sent with this ping.
    nonce -> Option<&str>,
  }
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
}
