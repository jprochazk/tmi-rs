//! Sent when a single message is deleted.

use super::{parse_message_text, parse_timestamp, MessageParseError};
use crate::common::{ChannelRef, MaybeOwned};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};
use std::borrow::Cow;

/// Sent when a single message is deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClearMsg<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: MaybeOwned<'src, ChannelRef>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  channel_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  sender: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  message_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,

  is_action: bool,

  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for ClearMsg<'src> as self {
    /// Login of the channel in which the message was deleted.
    channel -> &ChannelRef = self.channel.as_ref(),

    /// ID of the channel in which the message was deleted.
    channel_id -> &str = self.channel_id.as_ref(),

    /// Login of the user which sent the deleted message.
    sender -> &str = self.sender.as_ref(),

    /// Unique ID of the deleted message.
    message_id -> &str = self.message_id.as_ref(),

    /// Text of the deleted message.
    text -> &str = self.text.as_ref(),

    /// Whether the deleted message was sent with `/me`.
    is_action -> bool,

    /// Time at which the [`ClearMsg`] was executed on Twitch servers.
    timestamp -> DateTime<Utc>,
  }
}

impl<'src> ClearMsg<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::ClearMsg {
      return None;
    }

    let (text, is_action) = parse_message_text(message.text()?);
    Some(ClearMsg {
      channel: MaybeOwned::Ref(message.channel()?),
      channel_id: message.tag(Tag::RoomId)?.into(),
      sender: message.tag(Tag::Login)?.into(),
      message_id: message.tag(Tag::TargetMsgId)?.into(),
      text: text.into(),
      is_action,
      timestamp: parse_timestamp(message.tag(Tag::TmiSentTs)?)?,
    })
  }
}

impl<'src> super::FromIrc<'src> for ClearMsg<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<ClearMsg<'src>> for super::Message<'src> {
  fn from(msg: ClearMsg<'src>) -> Self {
    super::Message::ClearMsg(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_clearmsg_basic() {
    assert_irc_snapshot!(ClearMsg, "@login=alazymeme;room-id=;target-msg-id=3c92014f-340a-4dc3-a9c9-e5cf182f4a84;tmi-sent-ts=1594561955611 :tmi.twitch.tv CLEARMSG #pajlada :lole");
  }

  #[test]
  fn parse_clearmsg_action() {
    assert_irc_snapshot!(ClearMsg, "@login=alazymeme;room-id=;target-msg-id=3c92014f-340a-4dc3-a9c9-e5cf182f4a84;tmi-sent-ts=1594561955611 :tmi.twitch.tv CLEARMSG #pajlada :\u{0001}ACTION lole\u{0001}");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_clearmsg_basic() {
    assert_irc_roundtrip!(ClearMsg, "@login=alazymeme;room-id=;target-msg-id=3c92014f-340a-4dc3-a9c9-e5cf182f4a84;tmi-sent-ts=1594561955611 :tmi.twitch.tv CLEARMSG #pajlada :lole");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_clearmsg_action() {
    assert_irc_roundtrip!(ClearMsg, "@login=alazymeme;room-id=;target-msg-id=3c92014f-340a-4dc3-a9c9-e5cf182f4a84;tmi-sent-ts=1594561955611 :tmi.twitch.tv CLEARMSG #pajlada :\u{0001}ACTION lole\u{0001}");
  }
}
