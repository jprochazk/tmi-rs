//! ## IRCv3 Message parser
//!
//! The entrypoint to this module is [`IrcMessage`].
//!
//! ```rust,no_run
//! let msg = tmi::IrcMessage::parse("...");
//! ```
//!
//! ⚠ This parser is _not_ compliant with the IRCv3 spec!
//! It assumes that it will only ever parse messages sent by Twitch,
//! which means it handles Twitch-specific quirks, but it also means
//! that it's unlikely to work for IRC messages sent by other servers.

#![allow(dead_code)]

#[macro_use]
mod macros;

#[cfg(feature = "simd")]
mod simd;

mod scalar;

#[cfg(feature = "simd")]
use simd::{parse_prefix, parse_tags};

#[cfg(not(feature = "simd"))]
use scalar::{parse_prefix, parse_tags};

use crate::common::{ChannelRef, Span};
use std::fmt::{Debug, Display};

/// A base IRC message.
///
/// This variant references the original string instead of owning it.
#[derive(Clone)]
pub struct IrcMessageRef<'src> {
  src: &'src str,
  parts: IrcMessageParts,
}

#[derive(Clone)]
struct IrcMessageParts {
  tags: RawTags,
  prefix: Option<RawPrefix>,
  command: RawCommand,
  channel: Option<Span>,
  params: Option<Span>,
}

impl<'src> IrcMessageRef<'src> {
  /// Parse a single Twitch IRC message.
  pub fn parse(src: &'src str) -> Option<Self> {
    Self::parse_inner(src, Whitelist::<16, _>(whitelist_insert_all))
  }

  /// Parse a single Twitch IRC message with a tag whitelist.
  ///
  /// ```rust,ignore
  /// IrcMessageRef::parse_with_whitelist(
  ///     ":forsen!forsen@forsen.tmi.twitch.tv PRIVMSG #pajlada :AlienPls",
  ///     tmi::whitelist!(DisplayName, Id, TmiSentTs, UserId),
  /// )
  /// ```
  pub fn parse_with_whitelist<const IC: usize, F>(
    src: &'src str,
    whitelist: Whitelist<IC, F>,
  ) -> Option<Self>
  where
    F: Fn(&str, &mut RawTags, Span, Span),
  {
    Self::parse_inner(src, whitelist)
  }

  #[inline(always)]
  fn parse_inner<const IC: usize, F>(src: &'src str, whitelist: Whitelist<IC, F>) -> Option<Self>
  where
    F: Fn(&str, &mut RawTags, Span, Span),
  {
    let mut pos = 0usize;

    let tags = parse_tags(src, &mut pos, &whitelist);
    let prefix = parse_prefix(src, &mut pos);
    let command = parse_command(src, &mut pos)?;
    let channel = parse_channel(src, &mut pos);
    let params = parse_params(src, &pos);

    Some(Self {
      src,
      parts: IrcMessageParts {
        tags,
        prefix,
        command,
        channel,
        params,
      },
    })
  }

  /// Get the string from which this message was parsed.
  pub fn raw(&self) -> &'src str {
    self.src
  }

  /// Get an iterator over the message [`Tag`]s.
  pub fn tags(&self) -> impl Iterator<Item = (Tag<'src>, &'src str)> + '_ {
    self.parts.tags.iter().map(|pair| pair.get(self.src))
  }

  /// Get the message [`Prefix`].
  pub fn prefix(&self) -> Option<Prefix<'src>> {
    self.parts.prefix.map(|prefix| prefix.get(self.src))
  }

  /// Get the message [`Command`].
  pub fn command(&self) -> Command<'src> {
    self.parts.command.get(self.src)
  }

  /// Get the channel name this message was sent to.
  pub fn channel(&self) -> Option<&'src ChannelRef> {
    self
      .parts
      .channel
      .map(|span| &self.src[span])
      .map(ChannelRef::from_unchecked)
  }

  /// Get the raw message params.
  ///
  /// You have to call `split_whitespace` on it yourself.
  pub fn params(&self) -> Option<&'src str> {
    self.parts.params.map(|span| &self.src[span])
  }

  /// Retrieve the value of `tag`.
  ///
  /// `tag` can provided as:
  /// - A variant of the [`Tag`] enum
  /// - The stringified kebab-case tag name
  /// - [`Tag::Unknown`] with the stringified kebab-case tag name
  ///
  /// ⚠ [`Tag::Unknown`] has a different meaning from a specific
  /// [`Tag`] variant, or the kebab-case tag name, it will _not_
  /// match the others!
  ///
  /// ```rust,ignore
  /// assert!(message.tag(Tag::MsgId) == message.tag("msg-id"));
  /// assert!(message.tag(Tag::MsgId) != Tag::Unknown("msg-id"));
  /// assert!(message.tag("msg-id") != Tag::Unknown("msg-id"));
  /// ```
  pub fn tag<'a>(&self, tag: impl Into<Tag<'a>>) -> Option<&'src str> {
    let tag = tag.into();
    self
      .parts
      .tags
      .iter()
      .find(|RawTagPair(key, _)| key.get(self.src) == tag)
      .map(|RawTagPair(_, value)| &self.src[*value])
  }

  /// Returns the contents of the params after the last `:`.
  pub fn text(&self) -> Option<&'src str> {
    match self.parts.params {
      Some(params) => {
        let params = &self.src[params];
        match params.find(':') {
          Some(start) => Some(&params[start + 1..]),
          None => None,
        }
      }
      None => None,
    }
  }
}

impl<'src> Debug for IrcMessageRef<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Message")
      .field("tags", &DebugIter::new(self.tags()))
      .field("prefix", &self.prefix())
      .field("command", &self.command())
      .field("channel", &self.channel())
      .field("params", &self.params())
      .finish()
  }
}

/// A base IRC message.
///
/// This variants owns the input message.
pub struct IrcMessage {
  src: String,
  parts: IrcMessageParts,
}

impl IrcMessage {
  /// Parse a single Twitch IRC message.
  pub fn parse(src: impl ToString) -> Option<Self> {
    let src = src.to_string();
    let parts = IrcMessageRef::parse_inner(&src, Whitelist::<16, _>(whitelist_insert_all))?.parts;
    Some(IrcMessage { src, parts })
  }

  /// Parse a single Twitch IRC message with a tag whitelist.
  ///
  /// ```rust,ignore
  /// IrcMessage::parse_with_whitelist(
  ///     ":forsen!forsen@forsen.tmi.twitch.tv PRIVMSG #pajlada :AlienPls",
  ///     tmi::whitelist!(DisplayName, Id, TmiSentTs, UserId),
  /// )
  /// ```
  pub fn parse_with_whitelist<const IC: usize, F>(
    src: impl ToString,
    whitelist: Whitelist<IC, F>,
  ) -> Option<Self>
  where
    F: Fn(&str, &mut RawTags, Span, Span),
  {
    let src = src.to_string();
    let parts = IrcMessageRef::parse_inner(&src, whitelist)?.parts;
    Some(IrcMessage { src, parts })
  }

  /// Get the string from which this message was parsed.
  pub fn raw(&self) -> &str {
    &self.src
  }

  /// Get an iterator over the message [`Tag`]s.
  pub fn tags(&self) -> impl Iterator<Item = (Tag<'_>, &'_ str)> + '_ {
    self.parts.tags.iter().map(|pair| pair.get(&self.src))
  }

  /// Get the message [`Prefix`].
  pub fn prefix(&self) -> Option<Prefix<'_>> {
    self.parts.prefix.map(|prefix| prefix.get(&self.src))
  }

  /// Get the message [`Command`].
  pub fn command(&self) -> Command<'_> {
    self.parts.command.get(&self.src)
  }

  /// Get the channel name this message was sent to.
  pub fn channel(&self) -> Option<&str> {
    self.parts.channel.map(|span| &self.src.as_str()[span])
  }

  /// Get the raw message params.
  ///
  /// You have to call `split_whitespace` on it yourself.
  pub fn params(&self) -> Option<&str> {
    self.parts.params.map(|span| &self.src.as_str()[span])
  }

  /// Retrieve the value of `tag`.
  ///
  /// `tag` can provided as:
  /// - A variant of the [`Tag`] enum
  /// - The stringified kebab-case tag name
  /// - [`Tag::Unknown`] with the stringified kebab-case tag name
  ///
  /// ⚠ [`Tag::Unknown`] has a different meaning from a specific
  /// [`Tag`] variant, or the kebab-case tag name, it will _not_
  /// match the others!
  ///
  /// ```rust,ignore
  /// assert!(message.tag(Tag::MsgId) == message.tag("msg-id"));
  /// assert!(message.tag(Tag::MsgId) != Tag::Unknown("msg-id"));
  /// assert!(message.tag("msg-id") != Tag::Unknown("msg-id"));
  /// ```
  pub fn tag<'a>(&self, tag: impl Into<Tag<'a>>) -> Option<&str> {
    let tag = tag.into();
    self
      .parts
      .tags
      .iter()
      .find(|RawTagPair(key, _)| key.get(&self.src) == tag)
      .map(|RawTagPair(_, value)| &self.src.as_str()[*value])
  }

  /// Returns the contents of the params after the last `:`.
  pub fn text(&self) -> Option<&str> {
    match self.params() {
      Some(params) => match params.find(':') {
        Some(start) => Some(&params[start + 1..]),
        None => None,
      },
      None => None,
    }
  }
}

impl Debug for IrcMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("IrcMessage")
      .field("tags", &DebugIter::new(self.tags()))
      .field("prefix", &self.prefix())
      .field("command", &self.command())
      .field("channel", &self.channel())
      .field("params", &self.params())
      .finish()
  }
}

static_assert_send!(IrcMessageRef);
static_assert_sync!(IrcMessageRef);

static_assert_send!(IrcMessage);
static_assert_sync!(IrcMessage);

struct DebugIter<I>(std::cell::RefCell<I>);
impl<I> DebugIter<I> {
  fn new(iter: I) -> Self {
    Self(std::cell::RefCell::new(iter))
  }
}
impl<I> Debug for DebugIter<I>
where
  I: Iterator,
  I::Item: Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    use std::ops::DerefMut;
    let mut list = f.debug_list();
    for item in self.0.borrow_mut().deref_mut() {
      list.entry(&item);
    }
    list.finish()
  }
}

impl<'src> IrcMessageRef<'src> {
  /// Turn the [`IrcMessageRef`] into its owned variant, [`IrcMessage`].
  pub fn into_owned(self) -> IrcMessage {
    IrcMessage {
      src: self.src.into(),
      parts: self.parts.clone(),
    }
  }
}

impl IrcMessage {
  /// Turn the [`IrcMessage`] into its borrowed variant, [`IrcMessageRef`].
  pub fn as_ref(&self) -> IrcMessageRef<'_> {
    IrcMessageRef {
      src: &self.src,
      parts: self.parts.clone(),
    }
  }
}

/// Unescape a `value` according to the escaped characters that Twitch IRC supports.
///
/// Note that this is _not_ the same as IRCv3! Twitch doesn't follow the spec here.
pub fn unescape(value: &str) -> String {
  let mut out = String::with_capacity(value.len());
  let mut escape = false;
  for char in value.chars() {
    match char {
      ':' if escape => {
        out.push(';');
        escape = false;
      }
      's' if escape => {
        out.push(' ');
        escape = false;
      }
      '\\' if escape => {
        out.push('\\');
        escape = false;
      }
      'r' if escape => {
        out.push('\r');
        escape = false;
      }
      'n' if escape => {
        out.push('\n');
        escape = false;
      }
      '⸝' => out.push(','),
      '\\' => escape = true,
      c => out.push(c),
    }
  }
  out
}

/// A tag whitelist. Only the allowed tags will be parsed and stored.
pub struct Whitelist<const IC: usize, F>(F);

impl<const IC: usize, F> Whitelist<IC, F>
where
  F: Fn(&str, &mut RawTags, Span, Span),
{
  #[doc(hidden)]
  pub fn new(f: F) -> Self {
    Self(f)
  }

  #[doc(hidden)]
  #[inline(always)]
  pub(crate) fn maybe_insert(&self, src: &str, map: &mut RawTags, tag: Span, value: Span) {
    (self.0)(src, map, tag, value)
  }
}

#[inline(always)]
fn whitelist_insert_all(src: &str, map: &mut RawTags, tag: Span, value: Span) {
  map.push(RawTagPair(RawTag::parse(src, tag), value));
}

#[doc(hidden)]
#[derive(Clone)]
pub struct RawTagPair(pub RawTag, pub Span);

#[doc(hidden)]
pub type RawTags = Vec<RawTagPair>;

impl RawTagPair {
  #[doc(hidden)]
  #[inline]
  pub fn get<'src>(&self, src: &'src str) -> (Tag<'src>, &'src str) {
    (self.0.get(src), &src[self.1])
  }
}

#[derive(Clone, Copy)]
enum RawCommand {
  Ping,
  Pong,
  Join,
  Part,
  Privmsg,
  Whisper,
  Clearchat,
  Clearmsg,
  GlobalUserState,
  Notice,
  Reconnect,
  RoomState,
  UserNotice,
  UserState,
  Capability,
  RplWelcome,
  RplYourHost,
  RplCreated,
  RplMyInfo,
  RplNamReply,
  RplEndOfNames,
  RplMotd,
  RplMotdStart,
  RplEndOfMotd,
  Other(Span),
}

impl RawCommand {
  #[inline]
  fn get<'src>(&self, src: &'src str) -> Command<'src> {
    match self {
      RawCommand::Ping => Command::Ping,
      RawCommand::Pong => Command::Pong,
      RawCommand::Join => Command::Join,
      RawCommand::Part => Command::Part,
      RawCommand::Privmsg => Command::Privmsg,
      RawCommand::Whisper => Command::Whisper,
      RawCommand::Clearchat => Command::ClearChat,
      RawCommand::Clearmsg => Command::ClearMsg,
      RawCommand::GlobalUserState => Command::GlobalUserState,
      RawCommand::Notice => Command::Notice,
      RawCommand::Reconnect => Command::Reconnect,
      RawCommand::RoomState => Command::RoomState,
      RawCommand::UserNotice => Command::UserNotice,
      RawCommand::UserState => Command::UserState,
      RawCommand::Capability => Command::Capability,
      RawCommand::RplWelcome => Command::RplWelcome,
      RawCommand::RplYourHost => Command::RplYourHost,
      RawCommand::RplCreated => Command::RplCreated,
      RawCommand::RplMyInfo => Command::RplMyInfo,
      RawCommand::RplNamReply => Command::RplNames,
      RawCommand::RplEndOfNames => Command::RplEndOfNames,
      RawCommand::RplMotd => Command::RplMotd,
      RawCommand::RplMotdStart => Command::RplMotdStart,
      RawCommand::RplEndOfMotd => Command::RplEndOfMotd,
      RawCommand::Other(span) => Command::Other(&src[*span]),
    }
  }
}

/// A Twitch IRC command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command<'src> {
  /// Ping the peer
  Ping,
  /// The peer's response to a [`Command::Ping`]
  Pong,
  /// Join a channel
  Join,
  /// Leave a channel
  Part,
  /// Send a message to a channel
  Privmsg,
  /// Send a private message to a user
  Whisper,
  /// Purge a user's messages in a channel
  ClearChat,
  /// Remove a single message
  ClearMsg,
  /// Sent upon successful authentication (PASS/NICK command)
  GlobalUserState,
  /// General notices from the server
  Notice,
  /// Rejoins channels after a restart
  Reconnect,
  /// Identifies the channel's chat settings
  RoomState,
  /// Announces Twitch-specific events to the channel
  UserNotice,
  /// Identifies a user's chat settings or properties
  UserState,
  /// Requesting an IRC capability
  Capability,
  // Numeric commands
  /// `001`
  RplWelcome,
  /// `002`
  RplYourHost,
  /// `003`
  RplCreated,
  /// `004`
  RplMyInfo,
  /// `353`
  RplNames,
  /// `366`
  RplEndOfNames,
  /// `372`
  RplMotd,
  /// `375`
  RplMotdStart,
  /// `376`
  RplEndOfMotd,
  /// Unknown command
  Other(&'src str),
}

impl<'src> Display for Command<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

impl<'src> Command<'src> {
  /// Get the string value of the [`Command`].
  pub fn as_str(&self) -> &'src str {
    use Command::*;
    match self {
      Ping => "PING",
      Pong => "PONG",
      Join => "JOIN",
      Part => "PART",
      Privmsg => "PRIVMSG",
      Whisper => "WHISPER",
      ClearChat => "CLEARCHAT",
      ClearMsg => "CLEARMSG",
      GlobalUserState => "GLOBALUSERSTATE",
      Notice => "NOTICE",
      Reconnect => "RECONNECT",
      RoomState => "ROOMSTATE",
      UserNotice => "USERNOTICE",
      UserState => "USERSTATE",
      Capability => "CAP",
      RplWelcome => "001",
      RplYourHost => "002",
      RplCreated => "003",
      RplMyInfo => "004",
      RplNames => "353",
      RplEndOfNames => "366",
      RplMotd => "372",
      RplMotdStart => "375",
      RplEndOfMotd => "376",
      Other(cmd) => cmd,
    }
  }
}

macro_rules! tags_def {
  (
    $tag:ident, $raw_tag:ident, $tag_mod:ident;
    $($(#[$meta:meta])* $bytes:literal; $key:literal = $name:ident),*
  ) => {
    /// A parsed tag value.
    #[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
    #[non_exhaustive]
    pub enum $tag<'src> {
      $(
        $(#[$meta])*
        $name,
      )*
      Unknown(&'src str),
    }

    impl<'src> $tag<'src> {
      #[doc = concat!("Get the string value of the [`", stringify!($tag), "`].")]
      pub fn as_str(&self) -> &'src str {
        match self {
          $(Self::$name => $key,)*
          Self::Unknown(key) => key,
        }
      }

      #[doc = concat!("Parse a [`", stringify!($tag), "`] from a string.")]
      #[inline(never)]
      pub fn parse(src: &'src str) -> Self {
        match src.as_bytes() {
          $($bytes => Self::$name,)*
          _ => Self::Unknown(src),
        }
      }
    }

    #[doc(hidden)]
    #[derive(Clone, Copy)]
    #[non_exhaustive]
    pub enum $raw_tag {
      $($name,)*
      Unknown(Span),
    }

    impl $raw_tag {
      #[doc(hidden)]
      #[inline]
      fn get<'src>(&self, src: &'src str) -> $tag<'src> {
        match self {
          $(Self::$name => $tag::$name,)*
          Self::Unknown(span) => $tag::Unknown(&src[*span]),
        }
      }

      #[doc(hidden)]
      #[inline(never)]
      pub fn parse(src: &str, span: Span) -> Self {
        match src[span].as_bytes() {
          $($bytes => Self::$name,)*
          _ => Self::Unknown(span),
        }
      }
    }

    #[allow(non_upper_case_globals)]
    #[doc(hidden)]
    pub mod $tag_mod {
      $(pub const $name: &'static [u8] = $bytes;)*
    }
  }
}

impl<'src> From<&'src str> for Tag<'src> {
  fn from(value: &'src str) -> Self {
    Tag::parse(value)
  }
}

tags_def! {
  Tag, RawTag, tags;
  b"msg-id"; "msg-id" = MsgId,
  b"badges"; "badges" = Badges,
  b"badge-info"; "badge-info" = BadgeInfo,
  b"display-name"; "display-name" = DisplayName,
  b"emote-only"; "emote-only" = EmoteOnly,
  b"emotes"; "emotes" = Emotes,
  b"flags"; "flags" = Flags,
  b"id"; "id" = Id,
  b"mod"; "mod" = Mod,
  b"room-id"; "room-id" = RoomId,
  b"subscriber"; "subscriber" = Subscriber,
  b"tmi-sent-ts"; "tmi-sent-ts" = TmiSentTs,
  b"turbo"; "turbo" = Turbo,
  b"user-id"; "user-id" = UserId,
  b"user-type"; "user-type" = UserType,
  b"client-nonce"; "client-nonce" = ClientNonce,
  b"first-msg"; "first-msg" = FirstMsg,
  b"reply-parent-display-name"; "reply-parent-display-name" = ReplyParentDisplayName,
  b"reply-parent-msg-body"; "reply-parent-msg-body" = ReplyParentMsgBody,
  b"reply-parent-msg-id"; "reply-parent-msg-id" = ReplyParentMsgId,
  b"reply-parent-user-id"; "reply-parent-user-id" = ReplyParentUserId,
  b"reply-parent-user-login"; "reply-parent-user-login" = ReplyParentUserLogin,
  b"reply-thread-parent-msg-id"; "reply-thread-parent-msg-id" = ReplyThreadParentMsgId,
  b"reply-thread-parent-user-login"; "reply-thread-parent-user-login" = ReplyThreadParentUserLogin,
  b"followers-only"; "followers-only" = FollowersOnly,
  b"r9k"; "r9k" = R9K,
  b"rituals"; "rituals" = Rituals,
  b"slow"; "slow" = Slow,
  b"subs-only"; "subs-only" = SubsOnly,
  b"msg-param-cumulative-months"; "msg-param-cumulative-months" = MsgParamCumulativeMonths,
  b"msg-param-displayName"; "msg-param-displayName" = MsgParamDisplayName,
  b"msg-param-login"; "msg-param-login" = MsgParamLogin,
  b"msg-param-months"; "msg-param-months" = MsgParamMonths,
  b"msg-param-promo-gift-total"; "msg-param-promo-gift-total" = MsgParamPromoGiftTotal,
  b"msg-param-promo-name"; "msg-param-promo-name" = MsgParamPromoName,
  b"msg-param-recipient-display-name"; "msg-param-recipient-display-name" = MsgParamRecipientDisplayName,
  b"msg-param-recipient-id"; "msg-param-recipient-id" = MsgParamRecipientId,
  b"msg-param-recipient-user-name"; "msg-param-recipient-user-name" = MsgParamRecipientUserName,
  b"msg-param-sender-login"; "msg-param-sender-login" = MsgParamSenderLogin,
  b"msg-param-sender-name"; "msg-param-sender-name" = MsgParamSenderName,
  b"msg-param-should-share-streak"; "msg-param-should-share-streak" = MsgParamShouldShareStreak,
  b"msg-param-streak-months"; "msg-param-streak-months" = MsgParamStreakMonths,
  b"msg-param-sub-plan"; "msg-param-sub-plan" = MsgParamSubPlan,
  b"msg-param-sub-plan-name"; "msg-param-sub-plan-name" = MsgParamSubPlanName,
  b"msg-param-viewerCount"; "msg-param-viewerCount" = MsgParamViewerCount,
  b"msg-param-ritual-name"; "msg-param-ritual-name" = MsgParamRitualName,
  b"msg-param-threshold"; "msg-param-threshold" = MsgParamThreshold,
  b"msg-param-gift-months"; "msg-param-gift-months" = MsgParamGiftMonths,
  b"msg-param-color"; "msg-param-color" = MsgParamColor,
  b"login"; "login" = Login,
  b"bits"; "bits" = Bits,
  b"system-msg"; "system-msg" = SystemMsg,
  b"emote-sets"; "emote-sets" = EmoteSets,
  b"thread-id"; "thread-id" = ThreadId,
  b"message-id"; "message-id" = MessageId,
  b"returning-chatter"; "returning-chatter" = ReturningChatter,
  b"color"; "color" = Color,
  b"vip"; "vip" = Vip,
  b"target-user-id"; "target-user-id" = TargetUserId,
  b"target-msg-id"; "target-msg-id" = TargetMsgId,
  b"ban-duration"; "ban-duration" = BanDuration,
  b"msg-param-multimonth-duration"; "msg-param-multimonth-duration" = MsgParamMultimonthDuration,
  b"msg-param-was-gifted"; "msg-param-was-gifted" = MsgParamWasGifted,
  b"msg-param-multimonth-tenure"; "msg-param-multimonth-tenure" = MsgParamMultimonthTenure,
  b"sent-ts"; "sent-ts" = SentTs,
  b"msg-param-origin-id"; "msg-param-origin-id" = MsgParamOriginId,
  b"msg-param-fun-string"; "msg-param-fun-string" = MsgParamFunString,
  b"msg-param-sender-count"; "msg-param-sender-count" = MsgParamSenderCount,
  b"msg-param-profileImageURL"; "msg-param-profileImageURL" = MsgParamProfileImageUrl,
  b"msg-param-mass-gift-count"; "msg-param-mass-gift-count" = MsgParamMassGiftCount,
  b"msg-param-gift-month-being-redeemed"; "msg-param-gift-month-being-redeemed" = MsgParamGiftMonthBeingRedeemed,
  b"msg-param-anon-gift"; "msg-param-anon-gift" = MsgParamAnonGift
}

impl<'src> Display for Tag<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct RawPrefix {
  nick: Option<Span>,
  user: Option<Span>,
  host: Span,
}

impl RawPrefix {
  fn get<'src>(&self, src: &'src str) -> Prefix<'src> {
    Prefix {
      nick: self.nick.map(|span| &src[span]),
      user: self.user.map(|span| &src[span]),
      host: &src[self.host],
    }
  }
}

// TODO: have prefix only be two variants: `User` and `Host`
/// A message prefix.
///
/// ```text,ignore
/// :nick!user@host
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prefix<'src> {
  /// The `nick` part of the prefix.
  pub nick: Option<&'src str>,
  /// The `user` part of the prefix.
  pub user: Option<&'src str>,
  /// The `host` part of the prefix.
  pub host: &'src str,
}

impl<'src> std::fmt::Display for Prefix<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match (self.nick, self.user, self.host) {
      (Some(nick), Some(user), host) => write!(f, "{nick}!{user}@{host}"),
      (Some(nick), None, host) => write!(f, "{nick}@{host}"),
      (None, None, host) => write!(f, "{host}"),
      _ => Ok(()),
    }
  }
}

/// `COMMAND <rest>`
///
/// Returns `None` if command is unknown *and* empty
#[inline(always)]
fn parse_command(src: &str, pos: &mut usize) -> Option<RawCommand> {
  let (end, next_pos) = match src[*pos..].find(' ') {
    Some(end) => {
      let end = *pos + end;
      (end, end + 1)
    }
    None => (src.len(), src.len()),
  };

  use RawCommand as C;
  let cmd = match &src[*pos..end] {
    "PING" => C::Ping,
    "PONG" => C::Pong,
    "JOIN" => C::Join,
    "PART" => C::Part,
    "PRIVMSG" => C::Privmsg,
    "WHISPER" => C::Whisper,
    "CLEARCHAT" => C::Clearchat,
    "CLEARMSG" => C::Clearmsg,
    "GLOBALUSERSTATE" => C::GlobalUserState,
    "NOTICE" => C::Notice,
    "RECONNECT" => C::Reconnect,
    "ROOMSTATE" => C::RoomState,
    "USERNOTICE" => C::UserNotice,
    "USERSTATE" => C::UserState,
    "CAP" => C::Capability,
    "001" => C::RplWelcome,
    "002" => C::RplYourHost,
    "003" => C::RplCreated,
    "004" => C::RplMyInfo,
    "353" => C::RplNamReply,
    "366" => C::RplEndOfNames,
    "372" => C::RplMotd,
    "375" => C::RplMotdStart,
    "376" => C::RplEndOfMotd,
    other if !other.is_empty() => C::Other(Span::from(*pos..end)),
    _ => return None,
  };

  *pos = next_pos;

  Some(cmd)
}

/// #channel <rest>
#[inline(always)]
fn parse_channel(src: &str, pos: &mut usize) -> Option<Span> {
  match src[*pos..].starts_with('#') {
    true => {
      let start = *pos;
      match src[start..].find(' ') {
        Some(end) => {
          let end = start + end;
          *pos = end + 1;
          Some(Span::from(start..end))
        }
        None => {
          let end = src.len();
          *pos = end;
          Some(Span::from(start..end))
        }
      }
    }
    false => None,
  }
}

#[inline(always)]
fn parse_params(src: &str, pos: &usize) -> Option<Span> {
  if !src[*pos..].is_empty() {
    Some(Span::from(*pos..src.len()))
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod parse {
    use super::*;

    #[test]
    fn command() {
      let data = "PING <rest>";
      let mut pos = 0;

      let command = parse_command(data, &mut pos).unwrap();
      assert_eq!(command.get(data), Command::Ping);
      assert_eq!(&data[pos..], "<rest>");
    }

    #[test]
    fn channel() {
      let data = "#channel <rest>";
      let mut pos = 0;

      let channel = parse_channel(data, &mut pos).unwrap();
      assert_eq!(channel.get(data), "#channel");
      assert_eq!(&data[pos..], "<rest>");
    }

    #[test]
    fn params() {
      let data = ":param_a :param_b";
      let params = parse_params(data, &0).unwrap();
      assert_eq!(params.get(data), data)
    }

    #[test]
    fn notice_without_channel() {
      let data = ":tmi.twitch.tv NOTICE * :Improperly formatted auth";

      let msg = IrcMessageRef::parse(data).unwrap();
      assert_eq!(msg.command(), Command::Notice);
      assert_eq!(msg.text(), Some("Improperly formatted auth"));
      assert_eq!(msg.params(), Some("* :Improperly formatted auth"));
    }

    #[test]
    fn regression_parse_prefix() {
      let data = ":justinfan57624!justinfan57624@justinfan57624.tmi.twitch.tv JOIN #riotgames";

      let msg = IrcMessageRef::parse(data).unwrap();
      eprintln!("{:?}", msg.parts.prefix);
      assert_eq!(
        msg.prefix(),
        Some(Prefix {
          nick: Some("justinfan57624"),
          user: Some("justinfan57624"),
          host: "justinfan57624.tmi.twitch.tv"
        })
      );
    }

    #[test]
    fn regression_equals_in_tag_value() {
      let data = "@display-name=Dixtor334;emotes=;first-msg=0;flags=;id=0b4c70e4-9a47-4ce1-9c3e-8f78111cdc19;mod=0;reply-parent-display-name=minosura;reply-parent-msg-body=https://youtu.be/-ek4MFjz_eM?list=PL91C6439FD45DE2F3\\sannytfDinkDonk\\sstrimmer\\skorean\\sone;reply-parent-msg-id=7f811788-b897-4b4c-9f91-99fafe70eb7f;reply-parent-user-id=141993641;reply-parent-user-login=minosura;returning-chatter=0;room-id=56418014;subscriber=1;tmi-sent-ts=1686049636367;turbo=0;user-id=73714767;user-type= :dixtor334!dixtor334@dixtor334.tmi.twitch.tv PRIVMSG #anny :@minosura @anny";
      assert_eq!("https://youtu.be/-ek4MFjz_eM?list=PL91C6439FD45DE2F3\\sannytfDinkDonk\\sstrimmer\\skorean\\sone", IrcMessageRef::parse(data).unwrap().tag(Tag::ReplyParentMsgBody).unwrap());
    }
  }
}
