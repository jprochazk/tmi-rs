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

use std::fmt::Display;

#[derive(Debug)]
pub struct Message {
  raw: String,
  tags: Option<ParsedTags<'static>>,
  prefix: Option<Prefix<'static>>,
  command: Command<'static>,
  channel: Option<&'static str>,
  params: Option<&'static str>,
}

pub struct Whitelist<const IC: usize, F>(F);

impl<const IC: usize, F> Whitelist<IC, F>
where
  F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
{
  /// # Safety
  /// The callback `f` must guarantee not to leak any of its parameters.
  ///
  /// The easiest way to ensure safety is to use the `twitch::whitelist` macro.
  pub unsafe fn new(f: F) -> Self {
    Self(f)
  }

  #[inline(always)]
  pub(crate) fn maybe_insert(
    &self,
    map: &mut Tags<'static>,
    tag: &'static str,
    value: &'static str,
  ) {
    (self.0)(map, tag, value)
  }
}

#[inline(always)]
fn whitelist_insert_all(map: &mut Tags<'static>, tag: &'static str, value: &'static str) {
  map.push((Tag::parse(tag), value));
}

/// Parse a single Twitch IRC message.
///
/// If the message can't be parsed, the string is returned in the result as `Err`.
///
/// Twitch often sends multiple messages in a batch separated by `\r\n`.
/// Before parsing messages, you should always split them by `\r\n` first:
///
/// ```rust,ignore
/// if let Some(data) = ws.next().await {
///     if let Message::Text(data) = data? {
///         for message in data.lines().flat_map(twitch::parse) {
///             handle(message)
///         }
///     }
/// }
/// ```
pub fn parse(src: impl Into<String>) -> Result<Message, String> {
  Message::parse(src)
}

/// Parse a single Twitch IRC message with a tag whitelist.
///
/// ```rust,ignore
/// twitch::parse_with_whitelist(
///     ":forsen!forsen@forsen.tmi.twitch.tv PRIVMSG #pajlada :AlienPls",
///     twitch::whitelist!(DisplayName, Id, TmiSentTs, UserId),
/// )
/// ```
///
/// If the message can't be parsed, the string is returned in the result as `Err`.
///
/// Twitch often sends multiple messages in a batch separated by `\r\n`.
/// Before parsing messages, you should always split them by `\r\n` first:
///
/// ```rust,ignore
/// if let Some(data) = ws.next().await {
///     if let Message::Text(data) = data? {
///         for message in data.lines().flat_map(twitch::parse) {
///             handle(message)
///         }
///     }
/// }
/// ```
pub fn parse_with_whitelist<const IC: usize, F>(
  src: impl Into<String>,
  whitelist: Whitelist<IC, F>,
) -> Result<Message, String>
where
  F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
{
  Message::parse_with_whitelist(src, whitelist)
}

impl Message {
  pub fn parse(src: impl Into<String>) -> Result<Self, String> {
    Self::parse_inner(src.into(), Whitelist::<16, _>(whitelist_insert_all))
  }

  pub fn parse_with_whitelist<const IC: usize, F>(
    src: impl Into<String>,
    whitelist: Whitelist<IC, F>,
  ) -> Result<Self, String>
  where
    F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
  {
    Self::parse_inner(src.into(), whitelist)
  }

  fn parse_inner<const IC: usize, F>(
    raw: String,
    whitelist: Whitelist<IC, F>,
  ) -> Result<Self, String>
  where
    F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
  {
    let remainder = &raw[..];

    let (tags, remainder) = parse_tags(remainder, &whitelist);
    let (prefix, remainder) = parse_prefix(remainder);
    let (command, remainder) = match parse_command(remainder) {
      Some((command, remainder)) => (command, remainder),
      None => return Err(raw),
    };
    let (channel, remainder) = parse_channel(remainder);
    let params = parse_params(remainder);

    Ok(Self {
      raw,
      tags,
      prefix,
      command,
      channel,
      params,
    })
  }

  pub fn into_raw(self) -> String {
    self.raw
  }

  pub fn raw(&self) -> &str {
    &self.raw
  }

  pub fn tags(&self) -> Option<&[(Tag<'_>, &str)]> {
    self.tags.as_ref().map(|v| &v[..])
  }

  pub fn prefix(&self) -> Option<&Prefix<'_>> {
    self.prefix.as_ref()
  }

  pub fn command(&self) -> &Command<'_> {
    &self.command
  }

  pub fn channel(&self) -> Option<&str> {
    self.channel
  }

  pub fn params(&self) -> Option<&str> {
    self.params
  }

  pub fn tag(&self, tag: Tag<'_>) -> Option<&str> {
    self
      .tags
      .as_ref()
      .and_then(|map| map.iter().find(|(key, _)| key == &tag))
      .map(|(_, value)| value)
      .copied()
  }

  /// Returns the contents of the params after the last `:`.
  pub fn text(&self) -> Option<&str> {
    match &self.params {
      Some(params) => match params.find(':') {
        Some(start) => Some(&params[start + 1..]),
        None => None,
      },
      None => None,
    }
  }
}

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

#[inline(always)]
unsafe fn map_str_to_new_base(
  prev_base: &str,
  new_base: &str,
  existing: &'static str,
) -> &'static str {
  let start = existing.as_ptr() as usize - prev_base.as_ptr() as usize;
  debug_assert!(start + existing.len() <= new_base.len());
  let out = leak(new_base.get_unchecked(start..start + existing.len()));
  debug_assert!(existing == out);
  out
}

impl Clone for Message {
  fn clone(&self) -> Self {
    let prev_base = &self.raw;
    let new_base = self.raw.clone();

    let tags = match &self.tags {
      Some(tags) => {
        let mut out = Vec::new();
        out.reserve_exact(tags.len());
        for (tag, value) in tags.iter() {
          let tag = match tag {
            Tag::Unknown(s) => {
              Tag::Unknown(unsafe { map_str_to_new_base(prev_base, &new_base, s) })
            }
            tag => tag.private_clone(),
          };
          let value = unsafe { map_str_to_new_base(prev_base, &new_base, value) };
          out.push((tag, value));
        }
        Some(out.into_boxed_slice())
      }
      None => None,
    };

    let prefix = self.prefix.as_ref().map(|prefix| Prefix {
      nick: prefix
        .nick
        .map(|s| unsafe { map_str_to_new_base(prev_base, &new_base, s) }),
      user: prefix
        .user
        .map(|s| unsafe { map_str_to_new_base(prev_base, &new_base, s) }),
      host: unsafe { map_str_to_new_base(prev_base, &new_base, prefix.host) },
    });

    let command = match &self.command {
      Command::Unknown(s) => {
        Command::Unknown(unsafe { map_str_to_new_base(prev_base, &new_base, s) })
      }
      other => other.private_clone(),
    };

    let channel = self
      .channel
      .as_ref()
      .map(|channel| unsafe { map_str_to_new_base(prev_base, &new_base, channel) });

    let params = self
      .params
      .as_ref()
      .map(|params| unsafe { map_str_to_new_base(prev_base, &new_base, params) });

    Self {
      tags,
      prefix,
      command,
      channel,
      params,
      raw: new_base,
    }
  }
}

#[doc(hidden)]
pub type Tags<'src> = Vec<(Tag<'src>, &'src str)>;
#[doc(hidden)]
pub type ParsedTags<'src> = Box<[(Tag<'src>, &'src str)]>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command<'src> {
  Ping,
  Pong,
  /// Join channel
  Join,
  /// Leave channel
  Part,
  /// Twitch Private Message
  Privmsg,
  // Twitch extensions
  /// Send message to a single user
  Whisper,
  /// Purge a user's messages
  Clearchat,
  /// Single message removal
  Clearmsg,
  /// Sent upon successful authentication (PASS/NICK command)
  GlobalUserState,
  /// Channel starts or stops host mode
  HostTarget,
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
  RplWelcome,
  RplYourHost,
  RplCreated,
  RplMyInfo,
  RplNamReply,
  RplEndOfNames,
  RplMotd,
  RplMotdStart,
  RplEndOfMotd,
  /// Unknown command
  Unknown(&'src str),
}

impl<'src> Display for Command<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

impl<'src> Command<'src> {
  pub fn as_str(&self) -> &'src str {
    use Command::*;
    match self {
      Ping => "PING",
      Pong => "PONG",
      Join => "JOIN",
      Part => "PART",
      Privmsg => "PRIVMSG",
      Whisper => "WHISPER",
      Clearchat => "CLEARCHAT",
      Clearmsg => "CLEARMSG",
      GlobalUserState => "GLOBALUSERSTATE",
      HostTarget => "HOSTTARGET",
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
      RplNamReply => "353",
      RplEndOfNames => "366",
      RplMotd => "372",
      RplMotdStart => "375",
      RplEndOfMotd => "376",
      Unknown(cmd) => cmd,
    }
  }

  pub(crate) fn private_clone(&self) -> Self {
    use Command::*;
    match self {
      Ping => Ping,
      Pong => Pong,
      Join => Join,
      Part => Part,
      Privmsg => Privmsg,
      Whisper => Whisper,
      Clearchat => Clearchat,
      Clearmsg => Clearmsg,
      GlobalUserState => GlobalUserState,
      HostTarget => HostTarget,
      Notice => Notice,
      Reconnect => Reconnect,
      RoomState => RoomState,
      UserNotice => UserNotice,
      UserState => UserState,
      Capability => Capability,
      RplWelcome => RplWelcome,
      RplYourHost => RplYourHost,
      RplCreated => RplCreated,
      RplMyInfo => RplMyInfo,
      RplNamReply => RplNamReply,
      RplEndOfNames => RplEndOfNames,
      RplMotd => RplMotd,
      RplMotdStart => RplMotdStart,
      RplEndOfMotd => RplEndOfMotd,
      Unknown(cmd) => Unknown(cmd),
    }
  }
}

unsafe fn leak(s: &str) -> &'static str {
  unsafe { ::core::mem::transmute(s) }
}

macro_rules! tags_def {
  (
    $tag:ident, $tag_mod:ident;
    $($(#[$meta:meta])* $bytes:literal; $key:literal = $name:ident),*
  ) => {
    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub enum $tag<'src> {
      $(
        $(#[$meta])*
        $name,
      )*
      Unknown(&'src str),
    }

    #[allow(non_upper_case_globals)]
    #[doc(hidden)]
    pub mod $tag_mod {
      $(pub const $name: &'static [u8] = $bytes;)*
    }

    impl<'src> $tag<'src> {
      pub fn as_str(&self) -> &'src str {
        match self {
          $(Self::$name => $key,)*
          Self::Unknown(key) => key,
        }
      }

      pub fn parse(s: &'src str) -> Self {
        match s.as_bytes() {
          $($bytes => Self::$name,)*
          _ => Self::Unknown(s),
        }
      }

      pub(crate) fn private_clone(&self) -> Self {
        match self {
          $(Self::$name => Self::$name,)*
          Self::Unknown(key) => Self::Unknown(key),
        }
      }
    }
  }
}

tags_def! {
  Tag, tags;
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
  b"login"; "login" = Login,
  b"system-msg"; "system-msg" = SystemMsg,
  b"emote-sets"; "emote-sets" = EmoteSets,
  b"thread-id"; "thread-id" = ThreadId,
  b"message-id"; "message-id" = MessageId,
  b"returning-chatter"; "returning-chatter" = ReturningChatter,
  b"color"; "color" = Color,
  b"vip"; "vip" = Vip,
  b"target-user-id"; "target-user-id" = TargetUserId,
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

#[derive(Debug)]
pub struct Prefix<'src> {
  pub nick: Option<&'src str>,
  pub user: Option<&'src str>,
  pub host: &'src str,
}

/// `COMMAND <rest>`
///
/// Returns `None` if command is unknown *and* empty
fn parse_command(remainder: &str) -> Option<(Command<'static>, &str)> {
  let (cmd, remainder) = match remainder.split_once(' ') {
    Some(v) => v,
    None => (remainder, &remainder[remainder.len()..]),
  };

  use Command::*;
  let cmd = match cmd {
    "PING" => Ping,
    "PONG" => Pong,
    "JOIN" => Join,
    "PART" => Part,
    "PRIVMSG" => Privmsg,
    "WHISPER" => Whisper,
    "CLEARCHAT" => Clearchat,
    "CLEARMSG" => Clearmsg,
    "GLOBALUSERSTATE" => GlobalUserState,
    "HOSTTARGET" => HostTarget,
    "NOTICE" => Notice,
    "RECONNECT" => Reconnect,
    "ROOMSTATE" => RoomState,
    "USERNOTICE" => UserNotice,
    "USERSTATE" => UserState,
    "CAP" => Capability,
    "001" => RplWelcome,
    "002" => RplYourHost,
    "003" => RplCreated,
    "004" => RplMyInfo,
    "353" => RplNamReply,
    "366" => RplEndOfNames,
    "372" => RplMotd,
    "375" => RplMotdStart,
    "376" => RplEndOfMotd,
    other if !other.is_empty() => Unknown(unsafe { leak(cmd) }),
    _ => return None,
  };

  Some((cmd, remainder))
}

/// #channel <rest>
fn parse_channel(remainder: &str) -> (Option<&'static str>, &str) {
  if remainder.starts_with('#') {
    let (channel, remainder) = match remainder.split_once(' ') {
      Some(v) => v,
      None => (remainder, &remainder[remainder.len()..]),
    };

    // SAFETY: `channel` is a subslice of `base`.
    (Some(unsafe { &*(channel as *const _) }), remainder)
  } else {
    (None, remainder)
  }
}

fn parse_params(remainder: &str) -> Option<&'static str> {
  if !remainder.is_empty() {
    // SAFETY: `remainder` is a subslice of `base`.
    Some(unsafe { &*(remainder as *const _) })
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

      let (command, remainder) = parse_command(data).unwrap();
      assert_eq!(command, Command::Ping);
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn channel() {
      let data = "#channel <rest>";

      let (channel, remainder) = parse_channel(data);
      let channel = channel.unwrap();
      assert_eq!(channel, "#channel");
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn params() {
      let data = ":param_a :param_b";

      let params = parse_params(data);
      let params = params.unwrap();
      assert_eq!(params, data)
    }

    #[test]
    fn regression_equals_in_tag_value() {
      let data = "@display-name=Dixtor334;emotes=;first-msg=0;flags=;id=0b4c70e4-9a47-4ce1-9c3e-8f78111cdc19;mod=0;reply-parent-display-name=minosura;reply-parent-msg-body=https://youtu.be/-ek4MFjz_eM?list=PL91C6439FD45DE2F3\\sannytfDinkDonk\\sstrimmer\\skorean\\sone;reply-parent-msg-id=7f811788-b897-4b4c-9f91-99fafe70eb7f;reply-parent-user-id=141993641;reply-parent-user-login=minosura;returning-chatter=0;room-id=56418014;subscriber=1;tmi-sent-ts=1686049636367;turbo=0;user-id=73714767;user-type= :dixtor334!dixtor334@dixtor334.tmi.twitch.tv PRIVMSG #anny :@minosura @anny";

      let a = Message::parse(data).unwrap();
      let mut a = a
        .tags()
        .unwrap()
        .iter()
        .map(|(tag, value)| (tag.as_str(), unescape(value)))
        .collect::<Vec<_>>();
      a.sort_by_key(|(tag, _)| *tag);

      let b = twitch_irc::message::IRCMessage::parse(data).unwrap();
      let mut b = b
        .tags
        .0
        .iter()
        .map(|(tag, value)| (tag.as_str(), String::from(value.as_deref().unwrap_or(""))))
        .collect::<Vec<_>>();
      b.sort_by_key(|(tag, _)| *tag);

      assert_eq!(a, b);
    }
  }

  #[allow(clippy::redundant_clone)]
  #[test]
  fn clone_message() {
    let message = "@display-name=Dixtor334;emotes=;first-msg=0;flags=;id=0b4c70e4-9a47-4ce1-9c3e-8f78111cdc19;mod=0;reply-parent-display-name=minosura;reply-parent-msg-body=https://youtu.be/-ek4MFjz_eM?list=PL91C6439FD45DE2F3\\sannytfDinkDonk\\sstrimmer\\skorean\\sone;reply-parent-msg-id=7f811788-b897-4b4c-9f91-99fafe70eb7f;reply-parent-user-id=141993641;reply-parent-user-login=minosura;returning-chatter=0;room-id=56418014;subscriber=1;tmi-sent-ts=1686049636367;turbo=0;user-id=73714767;user-type= :dixtor334!dixtor334@dixtor334.tmi.twitch.tv PRIVMSG #anny :@minosura @anny";
    let message = parse(message).unwrap();
    let _ = message.clone();
  }
}
