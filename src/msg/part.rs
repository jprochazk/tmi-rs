//! Sent when a user leaves a channel.

use super::MessageParseError;
use crate::irc::{Command, IrcMessageRef};
use std::borrow::Cow;

/// Sent when a user leaves a channel.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Part<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  user: Cow<'src, str>,
}

generate_getters! {
  <'src> for Part<'src> as self {
    /// Parted channel name.
    channel -> &str = self.channel.as_ref(),

    /// Login of the user.
    user -> &str = self.user.as_ref(),
  }
}

impl<'src> Part<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Part {
      return None;
    }

    Some(Part {
      channel: message.channel()?.into(),
      user: message
        .prefix()
        .and_then(|prefix| prefix.nick)
        .map(Cow::Borrowed)?,
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

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_join() {
    assert_irc_roundtrip!(
      Part,
      ":randers811!randers811@randers811.tmi.twitch.tv PART #pajlada"
    );
  }
}
