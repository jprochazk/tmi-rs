//! Sent when the chat is cleared of a batch of messages.

use super::{parse_duration, parse_timestamp, MessageParseError};
use crate::common::Channel;
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Duration, Utc};

/// Sent when the chat is cleared of a batch of messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClearChat<'src> {
  channel: Channel<'src>,
  channel_id: &'src str,
  action: Action<'src>,
  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for ClearChat<'src> as self {
    /// Name of the affected channel.
    channel -> &Channel<'_> = &self.channel,

    /// ID of the affected channel.
    channel_id -> &str,

    /// The specific kind of [`Action`] that this command represents.
    action -> Action<'_>,

    /// Time at which the [`ClearChat`] was executed on Twitch servers.
    timestamp -> DateTime<Utc>,
  }
}

impl<'src> ClearChat<'src> {
  /// Get the target of this [`ClearChat`] command.
  ///
  /// This returns the user which was timed out or banned.
  #[inline]
  pub fn target(&self) -> Option<&str> {
    use Action as C;
    match &self.action {
      C::Clear => None,
      C::Ban(Ban { user, .. }) | C::TimeOut(TimeOut { user, .. }) => Some(user),
    }
  }
}

/// Represents the specific way in which the chat was cleared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action<'src> {
  /// The entire chat was cleared.
  Clear,

  /// A single user was banned, clearing only their messages.
  Ban(Ban<'src>),

  /// A single user was timed out, clearing only their messages.
  TimeOut(TimeOut<'src>),
}

impl<'src> Action<'src> {
  /// Returns `true` if the clear chat action is [`Clear`].
  ///
  /// [`Clear`]: Action::Clear
  #[inline]
  pub fn is_clear(&self) -> bool {
    matches!(self, Self::Clear)
  }

  /// Returns `true` if the clear chat action is [`Ban`].
  ///
  /// [`Ban`]: Action::Ban
  #[inline]
  pub fn is_ban(&self) -> bool {
    matches!(self, Self::Ban(..))
  }

  /// Returns `true` if the clear chat action is [`TimeOut`].
  ///
  /// [`TimeOut`]: Action::TimeOut
  #[inline]
  pub fn is_time_out(&self) -> bool {
    matches!(self, Self::TimeOut(..))
  }
}

/// A single user was banned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ban<'src> {
  user: &'src str,
  id: &'src str,
}

generate_getters! {
  <'src> for Ban<'src> as self {
    /// Login of the banned user.
    user -> &str,

    /// ID of the banned user.
    id -> &str,
  }
}

/// A single user was timed out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeOut<'src> {
  user: &'src str,
  id: &'src str,
  duration: Duration,
}

generate_getters! {
  <'src> for TimeOut<'src> as self {
    /// Login of the timed out user.
    user -> &str,

    /// ID of the timed out user.
    id -> &str,

    /// Duration of the timeout.
    duration -> Duration,
  }
}

impl<'src> ClearChat<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
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
        (Some(name), Some(duration)) => Action::TimeOut(TimeOut {
          user: name,
          id: message.tag(Tag::TargetUserId)?,
          duration,
        }),
        (Some(name), None) => Action::Ban(Ban {
          user: name,
          id: message.tag(Tag::TargetUserId)?,
        }),
        (None, _) => Action::Clear,
      },
      timestamp: parse_timestamp(message.tag(Tag::TmiSentTs)?)?,
    })
  }
}

impl<'src> super::FromIrc<'src> for ClearChat<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<ClearChat<'src>> for super::Message<'src> {
  fn from(msg: ClearChat<'src>) -> Self {
    super::Message::ClearChat(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_clearchat_timeout() {
    assert_irc_snapshot!(ClearChat, "@ban-duration=1;room-id=11148817;target-user-id=148973258;tmi-sent-ts=1594553828245 :tmi.twitch.tv CLEARCHAT #pajlada :fabzeef");
  }

  #[test]
  fn parse_clearchat_ban() {
    assert_irc_snapshot!(ClearChat, "@room-id=11148817;target-user-id=70948394;tmi-sent-ts=1594561360331 :tmi.twitch.tv CLEARCHAT #pajlada :weeb123");
  }

  #[test]
  fn parse_clearchat_clear() {
    assert_irc_snapshot!(
      ClearChat,
      "@room-id=40286300;tmi-sent-ts=1594561392337 :tmi.twitch.tv CLEARCHAT #randers"
    );
  }
}
