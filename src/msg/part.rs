//! Sent when a user leaves a channel.

use super::MessageParseError;
use crate::common::ChannelRef;
use crate::irc::{Command, IrcMessageRef};

/// Sent when a user leaves a channel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part<'src> {
  channel: &'src ChannelRef,
  user: &'src str,
}

generate_getters! {
  <'src> for Part<'src> as self {
    /// Parted channel name.
    channel -> &'src ChannelRef,

    /// Login of the user.
    user -> &'src str,
  }
}

impl<'src> Part<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Part {
      return None;
    }

    Some(Part {
      channel: message.channel()?,
      user: message.prefix().and_then(|prefix| prefix.nick)?,
    })
  }
}

impl<'src> super::FromIrc<'src> for Part<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
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
