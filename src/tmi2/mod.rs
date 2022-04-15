use std::collections::HashMap;

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

  pub fn tags(&mut self) -> Option<Tags> {
    self.tags.as_mut().map(|t| {
      let base = &self.raw;
      Tags {
        base,
        tags: t.get(base),
      }
    })
  }
}

pub struct Tags<'src> {
  base: &'src str,
  tags: &'src HashMap<Tag, Range>,
}

impl<'src> Tags<'src> {
  pub fn get(&mut self, key: Tag) -> Option<&'src str> {
    self.tags.get(&key).map(|pos| &self.base[pos.clone()])
  }
}

#[derive(Debug, PartialEq)]
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
      self.len(),
    ))
  }

  unsafe fn extend_right(self, n: usize) -> Self {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.as_ptr(), self.len() + n))
  }
}

struct TagsLazy {
  raw: Range,
  inner: Option<HashMap<Tag, Range>>,
}

impl TagsLazy {
  /// Get a map of `Tag -> Range`. The map is lazily initialized.
  fn get(&mut self, base: &str) -> &mut HashMap<Tag, Range> {
    match self.inner {
      Some(ref mut v) => v,
      None => {
        self.inner = Some(TagsLazy::parse_inner(self.raw.clone(), base));
        // SAFETY: we just assigned it as `Some`
        unsafe { self.inner.as_mut().unwrap_unchecked() }
      }
    }
  }

  fn parse_inner(raw: Range, base: &str) -> HashMap<Tag, Range> {
    let mut out = HashMap::new();
    for (key, value) in base.split(';').flat_map(|v| v.split_once('=')) {
      let key = TAG_MAP
        .get(key)
        .cloned()
        // This is a bit fucky, but I don't know of a better way
        // to do this without ruining the ergonomics of the API.
        .unwrap_or_else(|| panic!("Unrecognized tag key {key}: Are you running the latest version of twitch-rs? If yes, please create an issue."));
      // SAFETY: `value` is a subslice of `base`
      let value = unsafe { value.into_range(base) };
      out.insert(key, value);
    }
    out
  }
}

/// `@a=a;b=b;c= :<rest>`
fn parse_tags<'src>(base: &'src str, remainder: &'src str) -> (Option<TagsLazy>, &'src str) {
  if remainder.starts_with('@') {
    // propagating option here, because we want to immediately stop
    // if we can't find the ` :`, as we're under the assumption
    // that any message with tags also includes a prefix.
    let (tags, rest) = match remainder.split_once(" :") {
      Some(v) => v,
      None => return (None, remainder),
    };
    // SAFETY: we if the prefix exists in the condition above
    let tags = unsafe { tags.strip_prefix('@').unwrap_unchecked() };
    // we want split by " :", but preserve the ":", so we move the
    // slice's start to the left by 1 byte.
    // SAFETY: valid because `rest` has at least 2 chars in front of it (the " :").
    let remainder = unsafe { rest.extend_left(1) };
    // SAFETY: `tags` is a subslice of `raw`.
    let tags_raw = unsafe { tags.into_range(base) };
    let tags = TagsLazy {
      raw: tags_raw,
      inner: None,
    };
    (Some(tags), remainder)
  } else {
    (None, remainder)
  }
}

// TODO: list all tags
// Keep this in sync with `TAG_MAP`
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tag {
  Login,
  TmiSentTs,
}

lazy_static::lazy_static! {
  static ref TAG_MAP: HashMap<&'static str, Tag> = {
    let mut m = HashMap::new();
    m.insert("login", Tag::Login);
    m.insert("tmi-sent-ts", Tag::TmiSentTs);
    m
  };
}

struct Prefix {
  nick: Option<Range>,
  user: Option<Range>,
  host: Range,
}

/// `:nick!user@host <rest>`
fn parse_prefix<'src>(base: &'src str, remainder: &'src str) -> (Option<Prefix>, &'src str) {
  if remainder.starts_with(':') {
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
    // SAFETY: `other` is a subslice of `base`
    other if !other.is_empty() => Unknown(unsafe { other.into_range(base) }),
    _ => return None,
  };

  Some((cmd, remainder))
}

fn parse_channel<'src>(base: &'src str, remainder: &'src str) -> (Option<Range>, &'src str) {
  if remainder.starts_with('#') {
    let (channel, remainder) = match remainder.split_once(' ') {
      Some(v) => v,
      None => (remainder, &remainder[remainder.len()..]),
    };

    // SAFETY:
    // - `channel` has at least one character in front of it (the '#').
    // - `channel` is a subslice of `base`.
    let channel = unsafe { channel.extend_left(1).into_range(base) };
    (Some(channel), remainder)
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

// TODO: test
// - every `parse_XXX`
// - `Extend`, `SliceToRange`
// - test case with at least one occurrence of each command and tag,
//   with the correct format
