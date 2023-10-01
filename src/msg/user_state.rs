//! Sent upon joining a channel, or upon successfully sending a `PRIVMSG` message to a channel.
//!
//! This is like [`GlobalUserState`][crate::msg::global_user_state::GlobalUserState], but
//! carries channel-specific information.
//!
//! For example, [`UserState::badges`] may be different from [`GlobalUserState::badges`][crate::msg::global_user_state::GlobalUserState::badges].

use super::{is_not_empty, parse_badges, split_comma, Badge, MessageParseError};
use crate::common::Channel;
use crate::irc::{Command, IrcMessageRef, Tag};

/// Sent upon joining a channel, or upon successfully sending a `PRIVMSG` message to a channel.
///
/// This is like [`GlobalUserState`][crate::msg::global_user_state::GlobalUserState], but
/// carries channel-specific information.
///
/// For example, [`UserState::badges`] may be different from [`GlobalUserState::badges`][crate::msg::global_user_state::GlobalUserState::badges].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserState<'src> {
  channel: Channel<'src>,
  user_name: &'src str,
  badges: Vec<Badge<'src>>,
  emote_sets: Vec<&'src str>,
  color: Option<&'src str>,
}

generate_getters! {
  <'src> for UserState<'src> as self {
    /// Name of the channel in which this state applies to.
    channel -> Channel<'_>,

    /// Display name of the user.
    user_name -> &str,

    /// List of channel-specific badges.
    badges -> &[Badge<'_>] = self.badges.as_ref(),

    /// Emote sets which are available in this channel.
    emote_sets -> &[&str] = self.emote_sets.as_ref(),

    /// The user's selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str>,
  }
}

impl<'src> UserState<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::UserState {
      return None;
    }

    Some(UserState {
      channel: message.channel()?,
      user_name: message.tag(Tag::DisplayName)?,
      badges: message
        .tag(Tag::Badges)
        .zip(message.tag(Tag::BadgeInfo))
        .map(|(badges, badge_info)| parse_badges(badges, badge_info))
        .unwrap_or_default(),
      emote_sets: message
        .tag(Tag::EmoteSets)
        .map(split_comma)
        .map(Iterator::collect)
        .unwrap_or_default(),
      color: message.tag(Tag::Color).filter(is_not_empty),
    })
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
}
