use super::{parse_badges, parse_emotes, parse_message_text, parse_timestamp, Badge, Emote, User};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Privmsg<'src> {
  /// Channel in which this message was sent.
  pub channel: &'src str,

  /// ID of the channel in which this message was sent.
  pub channel_id: &'src str,

  /// Unique ID of the message.
  pub message_id: &'src str,

  /// Basic info about the user who sent this message.
  pub sender: User<'src>,

  /// Text content of the message.
  pub text: &'src str,

  /// Whether the message was sent with `/me`.
  pub is_action: bool,

  /// List of channel badges enabled by the user in the [channel][`Privmsg::channel`].
  pub badges: Vec<Badge<'src>>,

  /// The user's selected name color.
  ///
  /// [`None`] means the user has not selected a color.
  /// To match the behavior of Twitch, users should be
  /// given a globally-consistent random color.
  pub color: Option<&'src str>,

  /// The number of bits gifted with this message.
  pub bits: Option<u64>,

  /// The emote ranges present in this message.
  pub emotes: Vec<Emote<'src>>,

  /// The time at which the message was sent.
  pub timestamp: DateTime<Utc>,
}

impl<'src> super::FromIrc<'src> for Privmsg<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Privmsg {
      return None;
    }

    let (text, is_action) = parse_message_text(message.text()?);
    Some(Privmsg {
      channel: message.channel()?,
      channel_id: message.tag(Tag::RoomId)?,
      message_id: message.tag(Tag::Id)?,
      sender: User {
        id: message.tag(Tag::UserId)?,
        login: message.prefix().and_then(|prefix| prefix.nick)?,
        name: message.tag(Tag::DisplayName)?,
      },
      text,
      is_action,
      badges: parse_badges(message.tag(Tag::Badges)?, message.tag(Tag::BadgeInfo)?),
      color: message.tag(Tag::Color),
      bits: message.tag(Tag::Bits).and_then(|bits| bits.parse().ok()),
      emotes: message
        .tag(Tag::Emotes)
        .map(parse_emotes)
        .unwrap_or_default(),
      timestamp: message.tag(Tag::TmiSentTs).and_then(parse_timestamp)?,
    })
  }
}

impl<'src> From<Privmsg<'src>> for super::Message<'src> {
  fn from(msg: Privmsg<'src>) -> Self {
    super::Message::Privmsg(msg)
  }
}
