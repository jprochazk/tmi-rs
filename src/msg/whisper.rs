//! A direct message between users.

use super::is_not_empty;
use super::{parse_badges, Badge, User};
use crate::irc::{Command, IrcMessageRef, Tag};

/// A direct message between users.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Whisper<'src> {
  recipient: &'src str,
  sender: User<'src>,
  text: &'src str,
  badges: Vec<Badge<'src>>,
  emotes: &'src str,
  color: Option<&'src str>,
}

generate_getters! {
  <'src> for Whisper<'src> as self {
    /// Login of the recipient.
    recipient -> &str,

    /// Login of the sender.
    sender -> &User<'src> = &self.sender,

    /// Text content of the message.
    text -> &str,

    /// List of badges visible in the whisper window.
    badges -> &[Badge<'_>] = self.badges.as_ref(),

    /// The emote raw emote ranges present in this message.
    ///
    /// ⚠ Note: This is _hopelessly broken_ and should **never be used for any purpose whatsoever**,
    /// You should instead parse the emotes yourself out of the message according to the available emote sets.
    /// If for some reason you need it, here you go.
    raw_emotes -> &str = self.emotes.clone(),

    /// The [sender][`Whisper::sender`]'s selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str>,
  }
}

impl<'src> super::FromIrc<'src> for Whisper<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Whisper {
      return None;
    }

    let (recipient, text) = message.params()?.split_once(" :")?;

    Some(Whisper {
      recipient,
      sender: User {
        id: message.tag(Tag::UserId)?,
        login: message.prefix().and_then(|prefix| prefix.nick)?,
        name: message.tag(Tag::DisplayName)?.into(),
      },
      text,
      color: message.tag(Tag::Color).filter(is_not_empty),
      badges: message
        .tag(Tag::Badges)
        .zip(message.tag(Tag::BadgeInfo))
        .map(|(badges, badge_info)| parse_badges(badges, badge_info))
        .unwrap_or_default(),
      emotes: message.tag(Tag::Emotes).unwrap_or_default(),
    })
  }
}

impl<'src> From<Whisper<'src>> for super::Message<'src> {
  fn from(msg: Whisper<'src>) -> Self {
    super::Message::Whisper(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_whipser() {
    assert_irc_snapshot!(Whisper, "@badges=;color=#19E6E6;display-name=randers;emotes=25:22-26;message-id=1;thread-id=40286300_553170741;turbo=0;user-id=40286300;user-type= :randers!randers@randers.tmi.twitch.tv WHISPER randers811 :hello, this is a test Kappa");
  }
}
