// TODO: collect large amount of messages and get median number of badges
//       to have statistically significant value for use as the inline
//       slots in `SmallVec`
// TODO: `serde` derives under feature
// TODO: replace all `ok()?` with `and_then`

use crate::irc::{IrcMessage, IrcMessageRef};
use crate::Span;

impl IrcMessage {
  pub fn cast<'src, T: FromIrc<'src>>(&'src self) -> Option<T> {
    T::from_irc(self.as_ref())
  }
}

impl<'src> IrcMessageRef<'src> {
  pub fn cast<T: FromIrc<'src>>(self) -> Option<T> {
    T::from_irc(self)
  }
}

pub trait FromIrc<'src>: Sized {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self>;
}

#[derive(Clone, Debug)]
pub enum Message<'src> {
  ClearChat(ClearChat<'src>),
  ClearMsg(ClearMsg<'src>),
  GlobalUserState(GlobalUserState<'src>),
  Join(Join<'src>),
  Notice(Notice<'src>),
  Part(Part<'src>),
  Ping(Ping<'src>),
  Pong(Pong<'src>),
  Privmsg(Privmsg<'src>),
  Reconnect,
  RoomState(RoomState<'src>),
  UserNotice(UserNotice<'src>),
  UserState(UserState<'src>),
  Whisper(Whisper<'src>),
}

impl<'src> FromIrc<'src> for Message<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    use crate::irc::Command as C;
    let message = match message.command() {
      C::ClearChat => ClearChat::from_irc(message)?.into(),
      C::ClearMsg => ClearMsg::from_irc(message)?.into(),
      C::GlobalUserState => GlobalUserState::from_irc(message)?.into(),
      C::Join => Join::from_irc(message)?.into(),
      C::Notice => Notice::from_irc(message)?.into(),
      C::Part => Part::from_irc(message)?.into(),
      C::Ping => Ping::from_irc(message)?.into(),
      C::Pong => Pong::from_irc(message)?.into(),
      C::Privmsg => Privmsg::from_irc(message)?.into(),
      C::Reconnect => Self::Reconnect,
      C::RoomState => RoomState::from_irc(message)?.into(),
      C::UserNotice => UserNotice::from_irc(message)?.into(),
      C::UserState => UserState::from_irc(message)?.into(),
      C::Whisper => Whisper::from_irc(message)?.into(),
      _ => return None,
    };
    Some(message)
  }
}

type SmallVec<T, const N: usize> = smallvec::SmallVec<[T; N]>;

/// A chat badge.
#[derive(Clone, Debug)]
pub enum Badge<'src> {
  Staff,
  Turbo,
  Broadcaster,
  Moderator,
  Subscriber {
    version: &'src str,
    months: &'src str,
  },
  Other(BadgeData<'src>),
}

impl<'src> From<Badge<'src>> for BadgeData<'src> {
  fn from(value: Badge<'src>) -> Self {
    match value {
      Badge::Staff => BadgeData {
        name: "staff",
        version: "1",
        extra: None,
      },
      Badge::Turbo => BadgeData {
        name: "turbo",
        version: "1",
        extra: None,
      },
      Badge::Broadcaster => BadgeData {
        name: "broadcaster",
        version: "1",
        extra: None,
      },
      Badge::Moderator => BadgeData {
        name: "moderator",
        version: "1",
        extra: None,
      },
      Badge::Subscriber { version, months } => BadgeData {
        name: "subscriber",
        version,
        extra: Some(months),
      },
      Badge::Other(data) => data,
    }
  }
}

impl<'src> From<BadgeData<'src>> for Badge<'src> {
  fn from(value: BadgeData<'src>) -> Self {
    match value.name {
      "staff" => Self::Staff,
      "turbo" => Self::Turbo,
      "broadcaster" => Self::Broadcaster,
      "moderator" => Self::Moderator,
      "subscriber" => Self::Subscriber {
        version: value.version,
        months: value.extra.unwrap_or("0"),
      },
      _ => Self::Other(value),
    }
  }
}

#[derive(Clone, Debug)]
pub struct BadgeData<'src> {
  /// Name of the badge, e.g. `subscriber`.
  pub name: &'src str,

  /// Version of the badge,
  pub version: &'src str,

  /// Extra badge info, such as the exact number of
  /// subscribed months for `subscriber`.
  pub extra: Option<&'src str>,
}

#[derive(Clone, Debug)]
pub struct User<'src> {
  /// Id of the user.
  pub id: &'src str,

  /// Login of the user.
  pub login: &'src str,

  /// Display name.
  ///
  /// This is the name which appears in chat, and may contain arbitrary unicode characters.
  /// This is in contrast to [`User::login`] which is always only ASCII.
  pub name: &'src str,
}

#[derive(Clone, Debug)]
pub struct Emote<'src> {
  pub id: &'src str,
  ranges: &'src str,
}

impl<'src> Emote<'src> {
  pub fn ranges(&self) -> impl Iterator<Item = Span> + '_ {
    self
      .ranges
      .split(',')
      .flat_map(|range| range.split_once('-'))
      .flat_map(|(start, end)| {
        Some(Span {
          start: start.parse().ok()?,
          end: end.parse().ok()?,
        })
      })
  }
}

fn parse_timestamp(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
  use chrono::TimeZone;
  chrono::Utc.timestamp_millis_opt(s.parse().ok()?).single()
}

fn parse_duration(s: &str) -> Option<chrono::Duration> {
  Some(chrono::Duration::seconds(s.parse().ok()?))
}

fn parse_message_text(s: &str) -> (&str, bool) {
  let Some(s) = s.strip_prefix("\u{0001}ACTION ") else {
    return (s, false);
  };
  let Some(s) = s.strip_suffix('\u{0001}') else {
    return (s, false);
  };
  (s, true)
}

fn split_comma(s: &str) -> impl Iterator<Item = &str> + '_ {
  s.split(',')
}

fn parse_badges<'src>(badges: &'src str, badge_info: &'src str) -> SmallVec<Badge<'src>, 2> {
  if badges.is_empty() {
    return SmallVec::new();
  }

  let badge_info = badge_info
    .split(',')
    .flat_map(|info| info.split_once('/'))
    .collect::<SmallVec<_, 16>>();

  badges
    .split(',')
    .flat_map(|badge| badge.split_once('/'))
    .map(|(name, version)| match name {
      "staff" => Badge::Staff,
      "turbo" => Badge::Turbo,
      "broadcaster" => Badge::Broadcaster,
      "moderator" => Badge::Moderator,
      "subscriber" => Badge::Subscriber {
        version,
        months: badge_info
          .iter()
          .find(|(name, _)| *name == "subscriber")
          .map(|(_, value)| *value)
          .unwrap_or("0"),
      },
      name => Badge::Other(BadgeData {
        name,
        version,
        extra: badge_info
          .iter()
          .find(|(name, _)| *name == "subscriber")
          .map(|(_, value)| *value),
      }),
    })
    .collect()
}

fn parse_emotes(emotes: &str) -> Vec<Emote<'_>> {
  if emotes.is_empty() {
    return Vec::new();
  }

  emotes
    .split('/')
    .flat_map(|emote| emote.split_once(':'))
    .map(|(id, ranges)| Emote { id, ranges })
    .collect()
}

fn parse_bool(v: &str) -> bool {
  v.parse::<u8>().ok().map(|n| n > 0).unwrap_or(false)
}

pub mod clear_chat;
pub use clear_chat::*;
pub mod clear_msg;
pub use clear_msg::*;
pub mod global_user_state;
pub use global_user_state::*;
pub mod join;
pub use join::*;
pub mod notice;
pub use notice::*;
pub mod part;
pub use part::*;
pub mod ping;
pub use ping::*;
pub mod pong;
pub use pong::*;
pub mod privmsg;
pub use privmsg::*;
pub mod room_state;
pub use room_state::*;
pub mod user_notice;
pub use user_notice::*;
pub mod user_state;
pub use user_state::*;
pub mod whisper;
pub use whisper::*;
