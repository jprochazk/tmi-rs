use super::{parse_duration, parse_timestamp};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Duration, Utc};

/// Sent when the chat is cleared of a batch of messages.
#[derive(Clone, Debug)]
pub struct ClearChat<'src> {
  /// Name of the affected channel.
  pub channel: &'src str,

  /// ID of the affected channel.
  pub channel_id: &'src str,

  /// The specific kind of [`ClearChatAction`] that this command represents.
  pub action: ClearChatAction<'src>,

  /// Time at which the [`ClearChat`] was executed on Twitch servers.
  pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub enum ClearChatAction<'src> {
  /// The entire chat was cleared.
  Clear,

  /// A single user was banned.
  Ban {
    /// Login of the banned user.
    user: &'src str,

    /// ID of the banned user.
    id: &'src str,
  },

  /// A single user was timed out.
  TimeOut {
    /// Login of the timed out user.
    user: &'src str,

    /// ID of the timed out user.
    id: &'src str,

    /// Duration of the timeout.
    duration: Duration,
  },
}

impl<'src> super::FromIrc<'src> for ClearChat<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::ClearChat {
      return None;
    }

    Some(ClearChat {
      channel: message.channel()?,
      channel_id: message.tag(Tag::RoomId)?,
      action: match (
        message.text(),
        message.tag(Tag::BanDuration).and_then(parse_duration),
      ) {
        (Some(name), Some(duration)) => ClearChatAction::TimeOut {
          user: name,
          id: message.tag(Tag::TargetUserId)?,
          duration,
        },
        (Some(name), None) => ClearChatAction::Ban {
          user: name,
          id: message.tag(Tag::TargetUserId)?,
        },
        (None, _) => ClearChatAction::Clear,
      },
      timestamp: parse_timestamp(message.tag(Tag::TmiSentTs)?)?,
    })
  }
}

impl<'src> From<ClearChat<'src>> for super::Message<'src> {
  fn from(msg: ClearChat<'src>) -> Self {
    super::Message::ClearChat(msg)
  }
}
