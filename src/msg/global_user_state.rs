use super::{parse_badges, split_comma, Badge};
use crate::irc::{Command, IrcMessageRef, Tag};

/// This command is sent once upon successful login to TMI.
#[derive(Clone, Debug)]
pub struct GlobalUserState<'src> {
  /// ID of the logged in user.
  pub id: &'src str,

  /// Display name of the logged in user.
  ///
  /// This is the name which appears in chat, and may contain arbitrary unicode characters.
  /// It is separate from the user login, which is always only ASCII.
  pub name: &'src str,

  /// List of global badges.
  pub badges: Vec<Badge<'src>>,

  /// Emote sets which are available globally.
  pub emote_sets: Vec<&'src str>,

  /// Chat name color.
  ///
  /// [`None`] means the user has not selected a color.
  /// To match the behavior of Twitch, users should be
  /// given a globally-consistent random color.
  pub color: Option<&'src str>,
}

impl<'src> super::FromIrc<'src> for GlobalUserState<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::GlobalUserState {
      return None;
    }

    Some(GlobalUserState {
      id: message.tag(Tag::UserId)?,
      name: message.tag(Tag::DisplayName)?,
      badges: parse_badges(
        message.tag(Tag::Badges).unwrap_or_default(),
        message.tag(Tag::BadgeInfo).unwrap_or_default(),
      ),
      emote_sets: message
        .tag(Tag::EmoteSets)
        .map(split_comma)
        .map(Iterator::collect)
        .unwrap_or_default(),
      color: message.tag(Tag::Color),
    })
  }
}

impl<'src> From<GlobalUserState<'src>> for super::Message<'src> {
  fn from(msg: GlobalUserState<'src>) -> Self {
    super::Message::GlobalUserState(msg)
  }
}
