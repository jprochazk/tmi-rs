//! This command is sent once upon successful login to Twitch IRC.

use super::{is_not_empty, parse_badges, split_comma, Badge, MessageParseError};
use crate::common::{maybe_unescape, Cow};
use crate::irc::{Command, IrcMessageRef, Tag};

/// This command is sent once upon successful login to Twitch IRC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalUserState<'src> {
  id: &'src str,
  name: &'src str,
  badges: Vec<Badge<'src>>,
  emote_sets: Vec<&'src str>,
  color: Option<&'src str>,
}

generate_getters! {
  <'src> for GlobalUserState<'src> as self {
    /// ID of the logged in user.
    id -> &'src str,

    /// Display name of the logged in user.
    ///
    /// This is the name which appears in chat, and may contain arbitrary unicode characters.
    /// It is separate from the user login, which is always only ASCII.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    name -> Cow<'src, str> = maybe_unescape(self.name),

    /// List of global badges.
    badges -> &[Badge<'src>] = self.badges.as_ref(),

    /// Emote sets which are available globally.
    emote_sets -> &[&'src str] = self.emote_sets.as_ref(),

    /// Chat name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&'src str>,
  }
}

impl<'src> GlobalUserState<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::GlobalUserState {
      return None;
    }

    Some(GlobalUserState {
      id: message.tag(Tag::UserId)?,
      name: message.tag(Tag::DisplayName)?,
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
}
