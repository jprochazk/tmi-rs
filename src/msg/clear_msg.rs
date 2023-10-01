//! Sent when a single message is deleted.

use super::{parse_message_text, parse_timestamp};
use crate::common::Channel;
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};

/// Sent when a single message is deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClearMsg<'src> {
  channel: Channel<'src>,
  channel_id: &'src str,
  sender: &'src str,
  message_id: &'src str,
  text: &'src str,
  is_action: bool,
  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for ClearMsg<'src> as self {
    /// Login of the channel in which the message was deleted.
    channel -> Channel<'_>,

    /// ID of the channel in which the message was deleted.
    channel_id -> &str,

    /// Login of the user which sent the deleted message.
    sender -> &str,

    /// Unique ID of the deleted message.
    message_id -> &str,

    /// Text of the deleted message.
    text -> &str,

    /// Whether the deleted message was sent with `/me`.
    is_action -> bool,

    /// Time at which the [`ClearMsg`] was executed on Twitch servers.
    timestamp -> DateTime<Utc>,
  }
}

impl<'src> super::FromIrc<'src> for ClearMsg<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::ClearMsg {
      return None;
    }

    let (text, is_action) = parse_message_text(message.text()?);
    Some(ClearMsg {
      channel: message.channel()?,
      channel_id: message.tag(Tag::RoomId)?,
      sender: message.tag(Tag::Login)?,
      message_id: message.tag(Tag::TargetMsgId)?,
      text,
      is_action,
      timestamp: parse_timestamp(message.tag(Tag::TmiSentTs)?)?,
    })
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
}
