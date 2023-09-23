use super::{parse_message_text, parse_timestamp};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};

/// Sent when a single message is deleted.
#[derive(Clone, Debug)]
pub struct ClearMsg<'src> {
  /// Login of the channel in which the message was deleted.
  pub channel: &'src str,

  /// ID of the channel in which the message was deleted.
  pub channel_id: &'src str,

  /// Login of the user which sent the deleted message.
  pub sender: &'src str,

  /// Unique ID of the deleted message.
  pub message_id: &'src str,

  /// Text of the deleted message.
  pub text: &'src str,

  /// Whether the deleted message was sent with `/me`.
  pub is_action: bool,

  /// Time at which the [`ClearMsg`] was executed on Twitch servers.
  pub timestamp: DateTime<Utc>,
}

impl<'src> super::FromIrc<'src> for ClearMsg<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::ClearChat {
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
