//! Sent when the chat is cleared of a batch of messages.

use super::{parse_duration, parse_timestamp, MessageParseError};
use crate::common::{ChannelRef, MaybeOwned};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::time::Duration;

/// Sent when the chat is cleared of a batch of messages.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClearChat<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: MaybeOwned<'src, ChannelRef>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  channel_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  action: Action<'src>,

  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for ClearChat<'src> as self {
    /// Name of the affected channel.
    channel -> &ChannelRef = self.channel.as_ref(),

    /// ID of the affected channel.
    channel_id -> &str = self.channel_id.as_ref(),

    /// The specific kind of [`Action`] that this command represents.
    action -> &Action<'src> = &self.action,

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
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(rename_all = "lowercase")
)]
pub enum Action<'src> {
  /// The entire chat was cleared.
  Clear,

  /// A single user was banned, clearing only their messages.
  #[cfg_attr(feature = "serde", serde(borrow))]
  Ban(Ban<'src>),

  /// A single user was timed out, clearing only their messages.
  #[cfg_attr(feature = "serde", serde(borrow))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ban<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  user: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  id: Cow<'src, str>,
}

generate_getters! {
  <'src> for Ban<'src> as self {
    /// Login of the banned user.
    user -> &str = self.user.as_ref(),

    /// ID of the banned user.
    id -> &str = self.id.as_ref(),
  }
}

/// A single user was timed out.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeOut<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  user: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  id: Cow<'src, str>,

  duration: Duration,
}

generate_getters! {
  <'src> for TimeOut<'src> as self {
    /// Login of the timed out user.
    user -> &str = self.user.as_ref(),

    /// ID of the timed out user.
    id -> &str = self.id.as_ref(),

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
      channel: MaybeOwned::Ref(message.channel()?),
      channel_id: message.tag(Tag::RoomId)?.into(),
      action: match (
        message.text(),
        message.tag(Tag::BanDuration).and_then(parse_duration),
      ) {
        (Some(name), Some(duration)) => Action::TimeOut(TimeOut {
          user: name.into(),
          id: message.tag(Tag::TargetUserId)?.into(),
          duration,
        }),
        (Some(name), None) => Action::Ban(Ban {
          user: name.into(),
          id: message.tag(Tag::TargetUserId)?.into(),
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

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_clearchat_timeout() {
    assert_irc_roundtrip!(ClearChat, "@ban-duration=1;room-id=11148817;target-user-id=148973258;tmi-sent-ts=1594553828245 :tmi.twitch.tv CLEARCHAT #pajlada :fabzeef");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_clearchat_ban() {
    assert_irc_roundtrip!(ClearChat, "@room-id=11148817;target-user-id=70948394;tmi-sent-ts=1594561360331 :tmi.twitch.tv CLEARCHAT #pajlada :weeb123");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_clearchat_clear() {
    assert_irc_roundtrip!(
      ClearChat,
      "@room-id=40286300;tmi-sent-ts=1594561392337 :tmi.twitch.tv CLEARCHAT #randers"
    );
  }
}
