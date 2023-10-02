//! ## Twitch message types
//!
//! The entrypoint to this module is [`Message`].
//!
//! To convert an incoming [`IrcMessage`] into a [`Message`],
//! use [`IrcMessage::as_typed`].

// TODO: `serde` derives under feature

#[macro_use]
mod macros;

use crate::common::{maybe_unescape, Cow};
use crate::irc::{IrcMessage, IrcMessageRef};
use smallvec::SmallVec;

impl IrcMessage {
  /// Parses the base [`IrcMessage`] into a Twitch-specific [`Message`].
  pub fn as_typed(&self) -> Result<Message<'_>, MessageParseError> {
    Message::from_irc(self.as_ref())
  }
}

impl<'src> IrcMessageRef<'src> {
  /// Parses the base [`IrcMessage`] into a Twitch-specific [`Message`].
  pub fn as_typed(self) -> Result<Message<'src>, MessageParseError> {
    Message::from_irc(self)
  }
}

/// Implemented for types which may be parsed from a base [`IrcMessage`].
pub trait FromIrc<'src>: Sized + private::Sealed {
  /// Attempt to parse `Self` from an [`IrcMessage`].
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError>;
}

/// A fully parsed Twitch chat message.
///
/// Note that this one
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
  Other(IrcMessageRef<'src>),
}

impl<'src> Message<'src> {
  /// Attempt to parse a message from a string.
  ///
  /// This is shorthand for [`IrcMessageRef::parse`] followed by [`Message::from_irc`].
  pub fn parse(src: &'src str) -> Result<Self, MessageParseError> {
    IrcMessageRef::parse(src)
      .ok_or(MessageParseError)
      .and_then(Message::from_irc)
  }
}

/// Failed to parse a message.
#[derive(Clone, Copy, Debug)]
pub struct MessageParseError;
impl std::fmt::Display for MessageParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("failed to parse message")
  }
}
impl std::error::Error for MessageParseError {}

impl<'src> TryFrom<IrcMessageRef<'src>> for Message<'src> {
  type Error = MessageParseError;

  fn try_from(value: IrcMessageRef<'src>) -> Result<Self, Self::Error> {
    Message::from_irc(value)
  }
}

impl<'src> FromIrc<'src> for Message<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    use crate::irc::Command as C;
    Ok(match message.command() {
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
      _ => Message::Other(message),
    })
  }
}

/// A chat badge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Badge<'src> {
  /// `staff/1`
  Staff,
  /// `turbo/1`
  Turbo,
  /// `broadcaster/1`
  Broadcaster,
  /// `moderator/1`
  Moderator,
  /// `subscriber/{variant}` from `badges` + `subscriber/{months}` from `badge_info`.
  Subscriber(Subscriber<'src>),
  /// Some other badge.
  Other(BadgeData<'src>),
}

impl<'src> Badge<'src> {
  /// Get the base [`BadgeData`].
  pub fn as_badge_data(&self) -> BadgeData<'src> {
    BadgeData::from(self.clone())
  }
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
      Badge::Subscriber(Subscriber {
        version, months, ..
      }) => BadgeData {
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
      "subscriber" => Self::Subscriber(Subscriber {
        version: value.version,
        months: value.extra.unwrap_or("1"),
        months_n: value.extra.and_then(|v| v.parse().ok()).unwrap_or(1),
      }),
      _ => Self::Other(value),
    }
  }
}

/// A subscriber badge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscriber<'src> {
  version: &'src str,
  months: &'src str,
  months_n: u64,
}

generate_getters! {
  <'src> for Subscriber<'src> as self {
    /// Version of the badge.
    ///
    /// This comes from the `badges` tag.
    version -> &'src str,

    /// Number of months subscribed.
    ///
    /// This comes from the `badge_info` tag.
    months -> u64 = self.months_n,
  }
}

/// Basic info about a badge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadgeData<'src> {
  name: &'src str,
  version: &'src str,
  extra: Option<&'src str>,
}

generate_getters! {
  <'src> for BadgeData<'src> as self {
    /// Name of the badge, e.g. `subscriber`.
    ///
    /// This comes from the `badges` tag.
    name -> &'src str,

    /// Version of the badge.
    ///
    /// This comes from the `badges` tag.
    version -> &'src str,

    /// Extra badge info, such as the exact number of
    /// subscribed months for `subscriber`.
    ///
    /// This comes from the `badge_info` tag.
    extra -> Option<&'src str>,
  }
}

/// Basic information about a user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User<'src> {
  id: &'src str,
  login: &'src str,
  name: &'src str,
}

generate_getters! {
  <'src> for User<'src> as self {
    /// Id of the user.
    id -> &'src str,

    /// Login of the user.
    login -> &'src str,

    /// Display name.
    ///
    /// This is the name which appears in chat, and may contain arbitrary unicode characters.
    /// This is in contrast to [`User::login`] which is always only ASCII.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    name -> Cow<'src, str> = maybe_unescape(self.name),
  }
}

fn is_not_empty<T: AsRef<str>>(s: &T) -> bool {
  !s.as_ref().is_empty()
}

fn parse_timestamp(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
  use chrono::TimeZone;
  chrono::Utc.timestamp_millis_opt(s.parse().ok()?).single()
}

fn parse_duration(s: &str) -> Option<chrono::Duration> {
  Some(chrono::Duration::seconds(s.parse().ok()?))
}

fn parse_message_text(input: &str) -> (&str, bool) {
  let Some(s) = input.strip_prefix("\u{0001}ACTION ") else {
    return (input, false);
  };
  let Some(s) = s.strip_suffix('\u{0001}') else {
    return (input, false);
  };
  (s, true)
}

fn split_comma(s: &str) -> impl Iterator<Item = &str> + '_ {
  s.split(',')
}

fn parse_badges<'src>(badges: &'src str, badge_info: &'src str) -> Vec<Badge<'src>> {
  if badges.is_empty() {
    return Vec::new();
  }

  let badge_info = badge_info
    .split(',')
    .flat_map(|info| info.split_once('/'))
    .collect::<SmallVec<[_; 32]>>();

  badges
    .split(',')
    .flat_map(|badge| badge.split_once('/'))
    .map(|(name, version)| {
      BadgeData {
        name,
        version,
        extra: badge_info
          .iter()
          .find(|(needle, _)| *needle == name)
          .map(|(_, value)| *value),
      }
      .into()
    })
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

mod private {
  pub trait Sealed {}
}
impl private::Sealed for ClearChat<'_> {}
impl private::Sealed for ClearMsg<'_> {}
impl private::Sealed for GlobalUserState<'_> {}
impl private::Sealed for Join<'_> {}
impl private::Sealed for Notice<'_> {}
impl private::Sealed for Part<'_> {}
impl private::Sealed for Ping<'_> {}
impl private::Sealed for Pong<'_> {}
impl private::Sealed for Privmsg<'_> {}
impl private::Sealed for RoomState<'_> {}
impl private::Sealed for UserNotice<'_> {}
impl private::Sealed for UserState<'_> {}
impl private::Sealed for Whisper<'_> {}
impl private::Sealed for Message<'_> {}
