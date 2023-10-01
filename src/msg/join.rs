//! Sent when a user joins a channel.

use super::MessageParseError;
use crate::common::Channel;
use crate::irc::{Command, IrcMessageRef};

/// Sent when a user joins a channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Join<'src> {
  channel: Channel<'src>,
  user: &'src str,
}

generate_getters! {
  <'src> for Join<'src> as self {
    /// Joined channel name.
    channel -> Channel<'_>,

    /// Login of the user.
    user -> &str,
  }
}

impl<'src> Join<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Join {
      return None;
    }

    Some(Join {
      channel: message.channel()?,
      user: message.prefix().and_then(|prefix| prefix.nick)?,
    })
  }
}

impl<'src> super::FromIrc<'src> for Join<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<Join<'src>> for super::Message<'src> {
  fn from(msg: Join<'src>) -> Self {
    super::Message::Join(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_join() {
    assert_irc_snapshot!(
      Join,
      ":randers811!randers811@randers811.tmi.twitch.tv JOIN #pajlada"
    );
  }
}
