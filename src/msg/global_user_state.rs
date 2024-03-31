//! This command is sent once upon successful login to Twitch IRC.

use super::{is_not_empty, parse_badges, split_comma, Badge, MessageParseError};
use crate::common::maybe_unescape;
use crate::irc::{Command, IrcMessageRef, Tag};
use std::borrow::Cow;

/// This command is sent once upon successful login to Twitch IRC.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobalUserState<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  name: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  badges: Vec<Badge<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  emote_sets: Vec<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  color: Option<Cow<'src, str>>,
}

generate_getters! {
  <'src> for GlobalUserState<'src> as self {
    /// ID of the logged in user.
    id -> &str = self.id.as_ref(),

    /// Display name of the logged in user.
    ///
    /// This is the name which appears in chat, and may contain arbitrary unicode characters.
    /// It is separate from the user login, which is always only ASCII.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    name -> Cow<'src, str> = maybe_unescape(self.name.clone()),

    /// Iterator over global badges.
    badges -> impl DoubleEndedIterator<Item = &Badge<'src>> + ExactSizeIterator
      = self.badges.iter(),

    /// Number of global badges.
    num_badges -> usize = self.badges.len(),

    /// Iterator over emote sets which are available globally.
    emote_sets -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator
      = self.emote_sets.iter().map(|v| v.as_ref()),

    /// Number of emote sets which are available globally.
    num_emote_sets -> usize = self.emote_sets.len(),

    /// Chat name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str> = self.color.as_deref(),
  }
}

impl<'src> GlobalUserState<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::GlobalUserState {
      return None;
    }

    Some(GlobalUserState {
      id: message.tag(Tag::UserId)?.into(),
      name: message.tag(Tag::DisplayName)?.into(),
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
        .map(Cow::Borrowed),
    })
  }
}

impl<'src> super::FromIrc<'src> for GlobalUserState<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<GlobalUserState<'src>> for super::Message<'src> {
  fn from(msg: GlobalUserState<'src>) -> Self {
    super::Message::GlobalUserState(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_globaluserstate() {
    assert_irc_snapshot!(GlobalUserState, "@badge-info=;badges=;color=;display-name=randers811;emote-sets=0;user-id=553170741;user-type= :tmi.twitch.tv GLOBALUSERSTATE");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_globaluserstate() {
    assert_irc_roundtrip!(GlobalUserState, "@badge-info=;badges=;color=;display-name=randers811;emote-sets=0;user-id=553170741;user-type= :tmi.twitch.tv GLOBALUSERSTATE");
  }
}
