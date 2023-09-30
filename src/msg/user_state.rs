use super::is_not_empty;
use super::{parse_badges, split_comma, Badge};
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
      color: message.tag(Tag::Color).filter(is_not_empty),
    })
  }
}

impl<'src> From<UserState<'src>> for super::Message<'src> {
  fn from(msg: UserState<'src>) -> Self {
    super::Message::UserState(msg)
  }
}
/*
#[cfg(test)]
mod tests {
  use super::*;
  use crate::msg::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn parse_globaluserstate_new_user() {
    assert_irc_snapshot!("@badge-info=;badges=;color=;display-name=randers811;emote-sets=0;user-id=553170741;user-type= :tmi.twitch.tv GLOBALUSERSTATE");

  }
}
 */
