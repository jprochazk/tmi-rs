//! Sent by Twitch for various reasons to notify the client about something,
//! usually in response to invalid actions.

use super::MessageParseError;
use crate::irc::{Command, IrcMessageRef, Tag};
use std::borrow::Cow;

/// Sent by TMI for various reasons to notify the client about something,
/// usually in response to invalid actions.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notice<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: Option<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  id: Option<Cow<'src, str>>,
}

generate_getters! {
  <'src> for Notice<'src> as self {
    /// Target channel name.
    ///
    /// This may be empty before successful login.
    channel -> Option<&str> = self.channel.as_deref(),

    /// Notice message.
    text -> &str = self.text.as_ref(),

    /// Notice ID, see <https://dev.twitch.tv/docs/irc/msg-id/>.
    ///
    /// This will only be empty before successful login.
    id -> Option<&str> = self.id.as_deref(),
  }
}

impl<'src> Notice<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Notice {
      return None;
    }

    Some(Notice {
      channel: message.channel().map(Cow::Borrowed),
      text: message.text()?.into(),
      id: message.tag(Tag::MsgId).map(Cow::Borrowed),
    })
  }
}

impl<'src> super::FromIrc<'src> for Notice<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<Notice<'src>> for super::Message<'src> {
  fn from(msg: Notice<'src>) -> Self {
    super::Message::Notice(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_notice_before_login() {
    assert_irc_snapshot!(Notice, ":tmi.twitch.tv NOTICE * :Improperly formatted auth");
  }

  #[test]
  fn parse_notice_basic() {
    assert_irc_snapshot!(Notice, "@msg-id=msg_banned :tmi.twitch.tv NOTICE #forsen :You are permanently banned from talking in forsen.");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_notice_before_login() {
    assert_irc_roundtrip!(Notice, ":tmi.twitch.tv NOTICE * :Improperly formatted auth");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_notice_basic() {
    assert_irc_roundtrip!(Notice, "@msg-id=msg_banned :tmi.twitch.tv NOTICE #forsen :You are permanently banned from talking in forsen.");
  }
}
