//! A direct message between users.

use super::{is_not_empty, maybe_clone, parse_badges, Badge, MessageParseError, User};
use crate::irc::{Command, IrcMessageRef, Tag};
use std::borrow::Cow;

/// A direct message between users.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Whisper<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  recipient: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  sender: User<'src>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  badges: Vec<Badge<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  emotes: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  color: Option<Cow<'src, str>>,
}

generate_getters! {
  <'src> for Whisper<'src> as self {
    /// Login of the recipient.
    recipient -> &str = self.recipient.as_ref(),

    /// Login of the sender.
    sender -> User<'src>,

    /// Text content of the message.
    text -> &str = self.text.as_ref(),

    /// Iterator over the badges visible in the whisper window.
    badges -> impl DoubleEndedIterator<Item = &Badge<'src>> + ExactSizeIterator
      = self.badges.iter(),

    /// Number of badges visible in the whisper window.
    num_badges -> usize = self.badges.len(),

    /// The emote raw emote ranges present in this message.
    ///
    /// ⚠ Note: This is _hopelessly broken_ and should **never be used for any purpose whatsoever**,
    /// You should instead parse the emotes yourself out of the message according to the available emote sets.
    /// If for some reason you need it, here you go.
    raw_emotes -> &str = self.emotes.as_ref(),

    /// The [sender][`Whisper::sender`]'s selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str> = self.color.as_deref(),
  }
}

impl<'src> Whisper<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Whisper {
      return None;
    }

    let (recipient, text) = message.params()?.split_once(" :")?;

    Some(Whisper {
      recipient: recipient.into(),
      sender: User {
        id: message.tag(Tag::UserId)?.into(),
        login: message.prefix().and_then(|prefix| prefix.nick)?.into(),
        name: message.tag(Tag::DisplayName)?.into(),
      },
      text: text.into(),
      color: message
        .tag(Tag::Color)
        .filter(is_not_empty)
        .map(|v| v.into()),
      badges: message
        .tag(Tag::Badges)
        .zip(message.tag(Tag::BadgeInfo))
        .map(|(badges, badge_info)| parse_badges(badges, badge_info))
        .unwrap_or_default(),
      emotes: message.tag(Tag::Emotes).unwrap_or_default().into(),
    })
  }

  /// Convert this to a `'static` lifetime
  pub fn into_owned(self) -> Whisper<'static> {
    Whisper {
      recipient: maybe_clone(self.recipient),
      sender: self.sender.into_owned(),
      text: maybe_clone(self.text),
      badges: self.badges.into_iter().map(Badge::into_owned).collect(),
      emotes: maybe_clone(self.emotes),
      color: self.color.map(maybe_clone),
    }
  }
}

impl<'src> super::FromIrc<'src> for Whisper<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
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
  fn parse_whisper() {
    assert_irc_snapshot!(Whisper, "@badges=;color=#19E6E6;display-name=randers;emotes=25:22-26;message-id=1;thread-id=40286300_553170741;turbo=0;user-id=40286300;user-type= :randers!randers@randers.tmi.twitch.tv WHISPER randers811 :hello, this is a test Kappa");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_whisper() {
    assert_irc_roundtrip!(Whisper, "@badges=;color=#19E6E6;display-name=randers;emotes=25:22-26;message-id=1;thread-id=40286300_553170741;turbo=0;user-id=40286300;user-type= :randers!randers@randers.tmi.twitch.tv WHISPER randers811 :hello, this is a test Kappa");
  }
}
