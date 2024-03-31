//! Sent upon joining a channel, or upon successfully sending a `PRIVMSG` message to a channel.
//!
//! This is like [`GlobalUserState`][crate::msg::global_user_state::GlobalUserState], but
//! carries channel-specific information.
//!
//! For example, [`UserState::badges`] may be different from [`GlobalUserState::badges`][crate::msg::global_user_state::GlobalUserState::badges].

use super::{is_not_empty, maybe_clone, parse_badges, split_comma, Badge, MessageParseError};
use crate::irc::{Command, IrcMessageRef, Tag};
use std::borrow::Cow;

/// Sent upon joining a channel, or upon successfully sending a `PRIVMSG` message to a channel.
///
/// This is like [`GlobalUserState`][crate::msg::global_user_state::GlobalUserState], but
/// carries channel-specific information.
///
/// For example, [`UserState::badges`] may be different from [`GlobalUserState::badges`][crate::msg::global_user_state::GlobalUserState::badges].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UserState<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  user_name: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  badges: Vec<Badge<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  emote_sets: Vec<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  color: Option<Cow<'src, str>>,
}

generate_getters! {
  <'src> for UserState<'src> as self {
    /// Name of the channel in which this state applies to.
    channel -> &str = self.channel.as_ref(),

    /// Display name of the user.
    user_name -> &str = self.user_name.as_ref(),

    /// Iterator over channel-specific badges.
    badges -> impl DoubleEndedIterator<Item = &Badge<'src>> + ExactSizeIterator
      = self.badges.iter(),

    /// Number of channel-specific badges.
    num_badges -> usize = self.badges.len(),

    /// Iterator over the emote sets which are available in this channel.
    emote_sets -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator
      = self.emote_sets.iter().map(|v| v.as_ref()),

    /// Number of emote sets which are avaialble in this channel.
    num_emote_sets -> usize = self.emote_sets.len(),

    /// The user's selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str> = self.color.as_deref(),
  }
}

impl<'src> UserState<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::UserState {
      return None;
    }

    Some(UserState {
      channel: message.channel()?.into(),
      user_name: message.tag(Tag::DisplayName)?.into(),
      badges: message
        .tag(Tag::Badges)
        .zip(message.tag(Tag::BadgeInfo))
        .map(|(badges, badge_info)| parse_badges(badges, badge_info))
        .unwrap_or_default(),
      emote_sets: message
        .tag(Tag::EmoteSets)
        .map(split_comma)
        .map(|i| i.map(Cow::Borrowed))
        .map(Iterator::collect)
        .unwrap_or_default(),
      color: message
        .tag(Tag::Color)
        .filter(is_not_empty)
        .map(|v| v.into()),
    })
  }

  /// Clone data to give the value a `'static` lifetime.
  pub fn into_owned(self) -> UserState<'static> {
    UserState {
      channel: maybe_clone(self.channel),
      user_name: maybe_clone(self.user_name),
      badges: self.badges.into_iter().map(Badge::into_owned).collect(),
      emote_sets: self.emote_sets.into_iter().map(maybe_clone).collect(),
      color: self.color.map(maybe_clone),
    }
  }
}

impl<'src> super::FromIrc<'src> for UserState<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<UserState<'src>> for super::Message<'src> {
  fn from(msg: UserState<'src>) -> Self {
    super::Message::UserState(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_userstate() {
    assert_irc_snapshot!(UserState, "@badge-info=;badges=;color=#FF0000;display-name=TESTUSER;emote-sets=0;mod=0;subscriber=0;user-type= :tmi.twitch.tv USERSTATE #randers");
  }

  #[test]
  fn parse_userstate_uuid_emote_set_id() {
    assert_irc_snapshot!(UserState, "@badge-info=;badges=moderator/1;color=#8A2BE2;display-name=TESTUSER;emote-sets=0,75c09c7b-332a-43ec-8be8-1d4571706155;mod=1;subscriber=0;user-type=mod :tmi.twitch.tv USERSTATE #randers");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_userstate() {
    assert_irc_roundtrip!(UserState, "@badge-info=;badges=;color=#FF0000;display-name=TESTUSER;emote-sets=0;mod=0;subscriber=0;user-type= :tmi.twitch.tv USERSTATE #randers");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_userstate_uuid_emote_set_id() {
    assert_irc_roundtrip!(UserState, "@badge-info=;badges=moderator/1;color=#8A2BE2;display-name=TESTUSER;emote-sets=0,75c09c7b-332a-43ec-8be8-1d4571706155;mod=1;subscriber=0;user-type=mod :tmi.twitch.tv USERSTATE #randers");
  }
}
