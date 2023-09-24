use super::{parse_badges, split_comma, Badge};
use crate::irc::{Command, IrcMessageRef, Tag};

/// Sent upon joining a channel, or upon successfully sending a `PRIVMSG` message to a channel.
///
/// This is like [`GlobalUserState`][crate::msg::global_user_state::GlobalUserState], but
/// carries channel-specific information.
///
/// For example, [`UserState::badges`] may be different from [`GlobalUserState::badges`][crate::msg::global_user_state::GlobalUserState::badges].
#[derive(Clone, Debug)]
pub struct UserState<'src> {
  /// Name of the channel in which this state applies to.
  pub channel: &'src str,

  /// Display name of the user.
  pub user_name: &'src str,

  /// List of channel-specific badges.
  pub badges: Vec<Badge<'src>>,

  /// Emote sets which are available in this channel.
  pub emote_sets: Vec<&'src str>,

  /// The user's selected name color.
  ///
  /// [`None`] means the user has not selected a color.
  /// To match the behavior of Twitch, users should be
  /// given a globally-consistent random color.
  pub color: Option<&'src str>,
}

impl<'src> super::FromIrc<'src> for UserState<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::UserState {
      return None;
    }

    Some(UserState {
      channel: message.channel()?,
      user_name: message.tag(Tag::DisplayName)?,
      badges: parse_badges(message.tag(Tag::Badges)?, message.tag(Tag::BadgeInfo)?),
      emote_sets: message
        .tag(Tag::EmoteSets)
        .map(split_comma)
        .map(Iterator::collect)
        .unwrap_or_default(),
      color: message.tag(Tag::Color),
    })
  }
}

impl<'src> From<UserState<'src>> for super::Message<'src> {
  fn from(msg: UserState<'src>) -> Self {
    super::Message::UserState(msg)
  }
}
