//! Sent when a user leaves a channel.

use crate::common::Channel;
use crate::irc::{Command, IrcMessageRef};

/// Sent when a user leaves a channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part<'src> {
  channel: Channel<'src>,
  user: &'src str,
}

generate_getters! {
  <'src> for Part<'src> as self {
    /// Parted channel name.
    channel -> Channel<'_>,

    /// Login of the user.
    user -> &str,
  }
}

impl<'src> super::FromIrc<'src> for Part<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Part {
      return None;
    }

    Some(Part {
      channel: message.channel()?,
      user: message.prefix().and_then(|prefix| prefix.nick)?,
    })
  }
}

impl<'src> From<Part<'src>> for super::Message<'src> {
  fn from(msg: Part<'src>) -> Self {
    super::Message::Part(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_join() {
    assert_irc_snapshot!(
      Part,
      ":randers811!randers811@randers811.tmi.twitch.tv PART #pajlada"
    );
  }
}
