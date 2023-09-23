use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::Duration;

use super::parse_bool;

/// A partial update to the settings of some channel.
#[derive(Clone, Debug)]
pub struct RoomState<'src> {
  /// Login of the channel this state was applied to.
  pub channel: &'src str,

  /// ID of the channel this state was applied to.
  pub channel_id: &'src str,

  /// Whether the room is in emote-only mode.
  ///
  /// Chat messages may only contain emotes.
  ///
  /// - [`None`] means no change.
  /// - [`Some`] means enabled if `true`, and disabled if `false`.
  pub emote_only: Option<bool>,

  /// Whether the room is in followers-only mode.
  ///
  /// Only followers (optionally with a minimum followage) can chat.
  ///
  /// - [`None`] means no change.
  /// - [`Some`] means some change, see [`FollowersOnly`] for more information about possible values.
  pub followers_only: Option<FollowersOnly>,

  /// Whether the room is in r9k mode.
  ///
  /// Only unique messages may be sent to chat.
  pub r9k: Option<bool>,

  /// Whether the room is in slow mode.
  ///
  /// Users may only send messages with some minimum time between them.
  pub slow: Option<Duration>,

  /// Whether the room is in subcriber-only mode.
  ///
  /// Users may only send messages if they have an active subscription.
  pub subs_only: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum FollowersOnly {
  /// Followers-only mode is disabled.
  ///
  /// Anyone can send chat messages within the bounds
  /// of the other chat settings.
  Disabled,

  /// Followers-only mode is enabled, with an optional duration.
  ///
  /// If the duration is [`None`], then all followers can chat.
  /// Otherwise, only followers which have a follow age of at
  /// least the set duration can chat.
  Enabled(Option<Duration>),
}

impl<'src> super::FromIrc<'src> for RoomState<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::RoomState {
      return None;
    }

    Some(RoomState {
      channel: message.channel()?,
      channel_id: message.tag(Tag::RoomId)?,
      emote_only: message.tag(Tag::EmoteOnly).map(parse_bool),
      followers_only: message
        .tag(Tag::FollowersOnly)
        .and_then(|v| v.parse().ok())
        .map(|n| match n {
          n if n > 0 => FollowersOnly::Enabled(Some(Duration::seconds(n * 60))),
          0 => FollowersOnly::Enabled(None),
          _ => FollowersOnly::Disabled,
        }),
      r9k: message.tag(Tag::R9K).map(parse_bool),
      slow: message
        .tag(Tag::Slow)
        .and_then(|v| v.parse().ok())
        .map(Duration::seconds),
      subs_only: message.tag(Tag::SubsOnly).map(parse_bool),
    })
  }
}

impl<'src> From<RoomState<'src>> for super::Message<'src> {
  fn from(msg: RoomState<'src>) -> Self {
    super::Message::RoomState(msg)
  }
}
