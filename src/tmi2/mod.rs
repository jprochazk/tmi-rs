use std::{cell::UnsafeCell, collections::HashMap};

#[derive(Clone)]
pub struct Message {
  raw: String,
  tags: Option<TagsLazy>,
  prefix: Option<Prefix>,
  command: CommandRaw,
  channel: Option<Range>,
  params: Option<Range>,
}

impl Message {
  pub fn parse(src: impl Into<String>) -> Option<Self> {
    // rust-analyzer is not smart enough to infer `String` here...
    let raw: String = src.into();
    let remainder = &raw[..];

    let (tags, remainder) = parse_tags(&raw, remainder);
    let (prefix, remainder) = parse_prefix(&raw, remainder);
    let (command, remainder) = parse_command(&raw, remainder)?;
    let (channel, remainder) = parse_channel(&raw, remainder);
    let params = parse_params(&raw, remainder);

    Some(Self {
      raw,
      tags,
      prefix,
      command,
      channel,
      params,
    })
  }

  pub fn raw(&self) -> &str {
    &self.raw
  }

  pub fn tags(&self) -> Option<Tags> {
    self.tags.as_ref().map(|t| {
      let base = &self.raw;
      Tags {
        base,
        tags: t.get(base),
      }
    })
  }
}

#[derive(Clone, Copy)]
pub struct Tags<'src> {
  base: &'src str,
  tags: &'src HashMap<Tag, Range>,
}

impl<'src> Tags<'src> {
  pub fn get(&mut self, key: Tag) -> Option<&'src str> {
    self.tags.get(&key).map(|pos| &self.base[pos.clone()])
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

fn command_from_raw<'src>(raw: &'src CommandRaw, base: &'src str) -> Command<'src> {
  match raw {
    CommandRaw::Ping => Command::Ping,
    CommandRaw::Pong => Command::Pong,
    CommandRaw::Join => Command::Join,
    CommandRaw::Part => Command::Part,
    CommandRaw::Privmsg => Command::Privmsg,
    CommandRaw::Whisper => Command::Whisper,
    CommandRaw::Clearchat => Command::Clearchat,
    CommandRaw::Clearmsg => Command::Clearmsg,
    CommandRaw::GlobalUserState => Command::GlobalUserState,
    CommandRaw::HostTarget => Command::HostTarget,
    CommandRaw::Notice => Command::Notice,
    CommandRaw::Reconnect => Command::Reconnect,
    CommandRaw::RoomState => Command::RoomState,
    CommandRaw::UserNotice => Command::UserNotice,
    CommandRaw::UserState => Command::UserState,
    CommandRaw::Capability => Command::Capability,
    CommandRaw::RplWelcome => Command::RplWelcome,
    CommandRaw::RplYourHost => Command::RplYourHost,
    CommandRaw::RplCreated => Command::RplCreated,
    CommandRaw::RplMyInfo => Command::RplMyInfo,
    CommandRaw::RplNamReply => Command::RplNamReply,
    CommandRaw::RplEndOfNames => Command::RplEndOfNames,
    CommandRaw::RplMotd => Command::RplMotd,
    CommandRaw::RplMotdStart => Command::RplMotdStart,
    CommandRaw::RplEndOfMotd => Command::RplEndOfMotd,
    CommandRaw::Unknown(v) => Command::Unknown(&base[v.clone()]),
  }
}

type Range = std::ops::Range<usize>;

trait SliceToRange: Sized {
  /// Get the position (as a range) of `slice` in `base`.
  ///
  /// SAFETY: `slice` must be a subslice of `base`.
  unsafe fn into_range(self, base: &str) -> Range;
}

impl<'a> SliceToRange for &'a str {
  unsafe fn into_range(self, base: &str) -> Range {
    let start = self.as_ptr() as usize - base.as_ptr() as usize;
    let end = start + self.len();
    Range { start, end }
  }
}

trait Extend: Sized {
  /// Extend `slice` to the left by `n` bytes
  ///
  /// SAFETY:
  /// - This must not make the `slice` invalid utf-8
  /// - The resulting slice must still be a valid slice of the source string
  unsafe fn extend_left(self, n: usize) -> Self;
  /// Extend `slice` to the right by `n` bytes
  ///
  /// SAFETY:
  /// - This must not make the `slice` invalid utf-8
  /// - The resulting slice must still be a valid slice of the source string
  unsafe fn extend_right(self, n: usize) -> Self;
}

impl<'a> Extend for &'a str {
  unsafe fn extend_left(self, n: usize) -> Self {
    let count = -(n as isize);
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
      self.as_ptr().offset(count),
      self.len() + n,
    ))
  }

  unsafe fn extend_right(self, n: usize) -> Self {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(
      self.as_ptr(),
      (self.len() + n).saturating_sub(1),
    ))
  }
}

struct TagsLazy {
  raw: Range,
  inner: UnsafeCell<Option<HashMap<Tag, Range>>>,
}

impl TagsLazy {
  fn new(raw: Range) -> Self {
    TagsLazy {
      raw,
      inner: UnsafeCell::new(None),
    }
  }

  /// Get a map of `Tag -> Range`. The map is lazily initialized by parsing the tags.
  fn get(&self, base: &str) -> &HashMap<Tag, Range> {
    if !unsafe { &*self.inner.get() }.is_some() {
      // SAFETY: No references to the inner HashMap exist at this point in time
      // mutable or immutable, because it has not been initialized yet.
      // It is only initialiazed once, so this branch will never execute
      // if an immutable reference already exists.
      unsafe {
        self
          .inner
          .get()
          .write(Some(TagsLazy::parse_inner(self.raw.clone(), base)))
      };
    }
    let inner = unsafe { &*self.inner.get() };
    unsafe { inner.as_ref().unwrap_unchecked() }
  }

  fn parse_inner(raw: Range, base: &str) -> HashMap<Tag, Range> {
    let mut out = HashMap::new();
    let mut unrecognized = Vec::new();
    for (key, value) in base[raw].split(';').flat_map(|v| v.split_once('=')) {
      match TAGS.get(key).cloned() {
        Some(v) => {
          // SAFETY: `value` is a subslice of `base`
          out.insert(v, unsafe { value.into_range(base) });
        }
        None => unrecognized.push(key.to_string()),
      }
    }
    if !unrecognized.is_empty() {
      panic!("Unrecognized keys: {unrecognized:?}. Are you running the latest version of twitch-rs? If yes, please create an issue.");
    }
    out
  }
}

impl Clone for TagsLazy {
  fn clone(&self) -> Self {
    Self {
      raw: self.raw.clone(),
      inner: UnsafeCell::new(unsafe { &*self.inner.get() }.clone()),
    }
  }
}

/// `@a=a;b=b;c= :<rest>`
fn parse_tags<'src>(base: &'src str, remainder: &'src str) -> (Option<TagsLazy>, &'src str) {
  if let Some(remainder) = remainder.strip_prefix('@') {
    // propagating option here, because we want to immediately stop
    // if we can't find the ` :`, as we're under the assumption
    // that any message with tags also includes a prefix.
    let (tags, remainder) = match remainder.split_once(" :") {
      Some(v) => v,
      None => return (None, remainder),
    };
    // we want split by " :", but preserve the ":", so we move the
    // slice's start to the left by 1 byte.
    // SAFETY: valid because `rest` has at least 2 chars in front of it (the " :").
    let remainder = unsafe { remainder.extend_left(1) };
    // SAFETY: `tags` is a subslice of `raw`.
    let tags_raw = unsafe { tags.into_range(base) };
    (Some(TagsLazy::new(tags_raw)), remainder)
  } else {
    (None, remainder)
  }
}

macro_rules! tags_def {
  ($enum:ident $map:ident $($(#[$meta:meta])* $key:literal = $tag:ident),*) => {
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub enum $enum {
      $(
        $(#[$meta])*
        $tag
      ),*
    }

    lazy_static::lazy_static! {
      static ref $map: HashMap<&'static str, Tag> = {
        let mut m = HashMap::new();
        $(m.insert($key, Tag::$tag);)*
        m
      };
    }
  }
}

tags_def! { Tag TAGS
  // notice
  /// Test
  "msg-id" = MsgId,
  // privmsg
  /// Test
  "badges" = Badges,
  /// Test
  "badge-info" = BadgeInfo,
  /// Test
  "display-name" = DisplayName,
  /// Test
  "emote-only" = EmoteOnly,
  /// Test
  "emotes" = Emotes,
  /// Test
  "flags" = Flags,
  /// Test
  "id" = Id,
  /// Test
  "mod" = Mod,
  /// Test
  "room-id" = RoomId,
  /// Test
  "subscriber" = Subscriber,
  /// Test
  "tmi-sent-ts" = TmiSentTs,
  /// Test
  "turbo" = Turbo,
  /// Test
  "user-id" = UserId,
  /// Test
  "user-type" = UserType,
  /// Test
  "client-nonce" = ClientNonce,
  /// Test
  "first-msg" = FirstMsg,
  /// Test
  "reply-parent-display-name" = ReplyParentDisplayName,
  /// Test
  "reply-parent-msg-body" = ReplyParentMsgBody,
  /// Test
  "reply-parent-msg-id" = ReplyParentMsgId,
  /// Test
  "reply-parent-user-id" = ReplyParentUserId,
  /// Test
  "reply-parent-user-login" = ReplyParentUserLogin,
  // roomstate
  // "emote-only" = EmoteOnly,
  /// Test
  "followers-only" = FollowersOnly,
  /// Test
  "r9k" = R9K,
  /// Test
  "rituals" = Rituals,
  // "room-id" = RoomId,
  /// Test
  "slow" = Slow,
  /// Test
  "subs-only" = SubsOnly,
  // usernotice
  // "msg-id" = MsgId,
  /// Test
  "msg-param-cumulative-months" = MsgParamCumulativeMonths,
  /// Test
  "msg-param-displayName" = MsgParamDisplayName,
  /// Test
  "msg-param-login" = MsgParamLogin,
  /// Test
  "msg-param-months" = MsgParamMonths,
  /// Test
  "msg-param-promo-gift-total" = MsgParamPromoGiftTotal,
  /// Test
  "msg-param-promo-name" = MsgParamPromoName,
  /// Test
  "msg-param-recipient-display-name" = MsgParamRecipientDisplayName,
  /// Test
  "msg-param-recipient-id" = MsgParamRecipientId,
  /// Test
  "msg-param-recipient-user-name" = MsgParamRecipientUserName,
  /// Test
  "msg-param-sender-login" = MsgParamSenderLogin,
  /// Test
  "msg-param-sender-name" = MsgParamSenderName,
  /// Test
  "msg-param-should-share-streak" = MsgParamShouldShareStreak,
  /// Test
  "msg-param-streak-months" = MsgParamStreakMonths,
  /// Test
  "msg-param-sub-plan" = MsgParamSubPlan,
  /// Test
  "msg-param-sub-plan-name" = MsgParamSubPlanName,
  /// Test
  "msg-param-viewerCount" = MsgParamViewerCount,
  /// Test
  "msg-param-ritual-name" = MsgParamRitualName,
  /// Test
  "msg-param-threshold" = MsgParamThreshold,
  /// Test
  "msg-param-gift-months" = MsgParamGiftMonths,
  /// Test
  "login" = Login,
  /// Test
  "system-msg" = SystemMsg,
  // userstate
  /// Test
  "emote-sets" = EmoteSets,
  // whisper
  /// Test
  "thread-id" = ThreadId,
  /// Test
  "message-id" = MessageId
}

#[derive(Clone)]
struct Prefix {
  nick: Option<Range>,
  user: Option<Range>,
  host: Range,
}

/// `:nick!user@host <rest>`
fn parse_prefix<'src>(base: &'src str, remainder: &'src str) -> (Option<Prefix>, &'src str) {
  if let Some(remainder) = remainder.strip_prefix(':') {
    // :host <rest>
    // :nick@host <rest>
    // :nick!user@host <rest>
    let (prefix_raw, remainder) = match remainder.split_once(' ') {
      Some(v) => v,
      None => return (None, remainder),
    };

    let (nick, user, host) = match prefix_raw.split_once('@') {
      Some((nick_and_user, host)) => match nick_and_user.split_once('!') {
        // case: 'nick!user@host'
        Some((nick, user)) => (Some(nick), Some(user), host),
        // case: 'nick@host'
        None => (Some(nick_and_user), None, host),
      },
      // case: 'host'
      None => (None, None, prefix_raw),
    };

    let prefix = Prefix {
      // SAFETY: all of these are subslices of `base`.
      nick: nick.map(|v| unsafe { v.into_range(base) }),
      user: user.map(|v| unsafe { v.into_range(base) }),
      host: unsafe { host.into_range(base) },
    };
    (Some(prefix), remainder)
  } else {
    (None, remainder)
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CommandRaw {
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
  Unknown(Range),
}

/// `COMMAND <rest>`
///
/// Returns `None` if command is unknown *and* empty
fn parse_command<'src>(base: &'src str, remainder: &'src str) -> Option<(CommandRaw, &'src str)> {
  let (cmd, remainder) = match remainder.split_once(' ') {
    Some(v) => v,
    None => (remainder, &remainder[remainder.len()..]),
  };

  use CommandRaw::*;
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
    // SAFETY: `other` is a subslice of `base`
    other if !other.is_empty() => Unknown(unsafe { other.into_range(base) }),
    _ => return None,
  };

  Some((cmd, remainder))
}

/// #channel <rest>
fn parse_channel<'src>(base: &'src str, remainder: &'src str) -> (Option<Range>, &'src str) {
  if remainder.starts_with('#') {
    let (channel, remainder) = match remainder.split_once(' ') {
      Some(v) => v,
      None => (remainder, &remainder[remainder.len()..]),
    };

    // SAFETY: `channel` is a subslice of `base`.
    (Some(unsafe { channel.into_range(base) }), remainder)
  } else {
    (None, remainder)
  }
}

fn parse_params<'src>(base: &'src str, remainder: &'src str) -> Option<Range> {
  if !remainder.is_empty() {
    // SAFETY: `remainder` is a subslice of `base`.
    Some(unsafe { remainder.into_range(base) })
  } else {
    None
  }
}

// https://git.kotmisia.pl/Mm2PL/docs
// TODO: rework connection to TMI, try to make it runtime agnostic and more reliable
// TODO: test
// - every `parse_XXX`
// - test case with at least one occurrence of each command and tag, with the correct format

// TODO: two layers
// - low-level: tmi, pubsub, helix, eventsub
// - high-level:
//   - command registry
//   - output:
//     - chat for simple responses
//     - websocket, custom events that can be used to trigger anything
//   - API for adding/removing commands

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn str_extend_left() {
    let a = "test";
    let b = &a[1..];
    assert_eq!(a, unsafe { b.extend_left(1) });
  }

  #[test]
  fn str_extend_right() {
    let a = "test";
    let b = &a[..a.len()];
    assert_eq!(a, unsafe { b.extend_right(1) });
  }

  #[test]
  fn to_range() {
    let a = "test";
    let b = unsafe { a.into_range(a) };
    assert_eq!(a, &a[b]);
  }

  mod parse {
    use super::*;

    #[test]
    fn tags() {
      let data = "@login=test;id=asdf :<rest>";

      let (tags, remainder) = parse_tags(data, data);
      assert_eq!(remainder, &data[20..]);
      let tags = tags.unwrap();
      assert_eq!(tags.raw, Range { start: 1, end: 19 });

      let map = tags.get(data);
      assert_eq!(
        map,
        &[
          (Tag::Login, Range { start: 7, end: 11 }),
          (Tag::Id, Range { start: 15, end: 19 })
        ]
        .into_iter()
        .collect()
      )
    }

    #[test]
    fn prefix() {
      let data = ":nick!user@host <rest>";

      let (prefix, remainder) = parse_prefix(data, data);
      assert_eq!(remainder, &data[16..]);
      let prefix = prefix.unwrap();
      assert_eq!(&data[prefix.nick.unwrap()], "nick");
      assert_eq!(&data[prefix.user.unwrap()], "user");
      assert_eq!(&data[prefix.host], "host");
      assert_eq!(remainder, "<rest>");

      let data = ":nick@host <rest>";
      let (prefix, remainder) = parse_prefix(data, data);
      assert_eq!(remainder, &data[11..]);
      let prefix = prefix.unwrap();
      assert_eq!(&data[prefix.nick.unwrap()], "nick");
      assert!(prefix.user.is_none());
      assert_eq!(&data[prefix.host], "host");
      assert_eq!(remainder, "<rest>");

      let data = ":host <rest>";
      let (prefix, remainder) = parse_prefix(data, data);
      assert_eq!(remainder, &data[6..]);
      let prefix = prefix.unwrap();
      assert!(prefix.nick.is_none());
      assert!(prefix.user.is_none());
      assert_eq!(&data[prefix.host], "host");
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn command() {
      let data = "PING <rest>";

      let (command, remainder) = parse_command(data, data).unwrap();
      assert_eq!(command, CommandRaw::Ping);
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn channel() {
      let data = "#channel <rest>";

      let (channel, remainder) = parse_channel(data, data);
      let channel = &data[channel.unwrap()];
      assert_eq!(channel, "#channel");
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn params() {
      let data = ":param_a :param_b";

      let params = parse_params(data, data);
      let params = &data[params.unwrap()];
      assert_eq!(params, data)
    }
  }
}
