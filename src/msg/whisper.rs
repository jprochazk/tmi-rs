use super::{parse_badges, parse_emotes, Badge, Emote, SmallVec, User};
use crate::irc::{Command, IrcMessageRef, Tag};

/// A direct message between users.
#[derive(Clone, Debug)]
pub struct Whisper<'src> {
  /// Login of the recipient.
  pub recipient: &'src str,

  /// Login of the sender.
  pub sender: User<'src>,

  /// Text content of the message.
  pub text: &'src str,

  /// List of badges visible in the whisper window.
  pub badges: SmallVec<Badge<'src>, 2>,

  /// The emote ranges present in this message.
  pub emotes: Vec<Emote<'src>>,

  /// The [sender][`Whisper::sender`]'s selected name color.
  ///
  /// [`None`] means the user has not selected a color.
  /// To match the behavior of Twitch, users should be
  /// given a globally-consistent random color.
  pub color: Option<&'src str>,
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
        name: message.tag(Tag::DisplayName)?,
      },
      text,
      color: message.tag(Tag::Color),
      badges: parse_badges(message.tag(Tag::Badges)?, message.tag(Tag::BadgeInfo)?),
      emotes: message
        .tag(Tag::Emotes)
        .map(parse_emotes)
        .unwrap_or_default(),
    })
  }
}

impl<'src> From<Whisper<'src>> for super::Message<'src> {
  fn from(msg: Whisper<'src>) -> Self {
    super::Message::Whisper(msg)
  }
}
