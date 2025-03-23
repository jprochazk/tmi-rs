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

mod channel;
mod command;
mod params;
mod prefix;
mod tags;

#[cfg(feature = "simd")]
mod wide;

pub use command::Command;
pub use prefix::Prefix;
pub use tags::Tag;

use crate::common::Span;
use std::fmt::Debug;

use command::RawCommand;
use prefix::RawPrefix;
use tags::RawTags;

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
    Self::parse_inner(src)
  }

  #[inline(always)]
  fn parse_inner(src: &'src str) -> Option<Self> {
    let mut pos = 0usize;

    let tags = tags::parse(src, &mut pos).unwrap_or_default();
    let prefix = prefix::parse(src, &mut pos);
    let command = command::parse(src, &mut pos)?;
    let channel = channel::parse(src, &mut pos);
    let params = params::parse(src, &pos);

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
  pub fn tags(&self) -> impl Iterator<Item = (&'src str, &'src str)> + '_ {
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
  pub fn channel(&self) -> Option<&'src str> {
    self.parts.channel.map(|span| &self.src[span])
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
      .find(|pair| &self.src[pair.key()] == tag.as_str())
      .map(|pair| &self.src[pair.value()])
  }

  /// Returns the contents of the params after the last `:`.
  ///
  /// If `:` is not present, returns all params.
  pub fn text(&self) -> Option<&'src str> {
    match self.parts.params {
      Some(params) => {
        let params = &self.src[params];
        match params.find(':') {
          Some(start) => Some(&params[start + 1..]),
          None => Some(params),
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
    let parts = IrcMessageRef::parse_inner(&src)?.parts;
    Some(IrcMessage { src, parts })
  }

  /// Get the string from which this message was parsed.
  pub fn raw(&self) -> &str {
    &self.src
  }

  /// Get an iterator over the message [`Tag`]s.
  pub fn tags(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
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
      .find(|pair| &self.src.as_str()[pair.key()] == tag.as_str())
      .map(|pair| &self.src.as_str()[pair.value()])
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

// @key=value;key=value;key=value

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

#[cfg(test)]
mod tests {
  use super::*;

  mod parse {
    use super::*;

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

    #[test]
    fn regression_equals_in_tag_value_2() {
      // this one has the equals aligned differently from previous
      let data = "@room-id=11148817;tmi-sent-ts=1723702053033;color=#B7B6F9;reply-parent-msg-body=@RomeoGiggleToess\\shttps://www.youtube.com/watch?v=khMb3k-Wwvg;emotes=;flags=;reply-parent-user-id=53888434;id=96a5fb70-f54e-4640-979e-529a76ddf74b;reply-thread-parent-display-name=RomeoGiggleToess;reply-thread-parent-msg-id=fd2a5663-00cb-4e78-9c0d-aff6b66285bf;subscriber=0;historical=1;reply-parent-display-name=OGprodigy;mod=0;badges=twitch-dj/1;first-msg=0;user-id=86336791;reply-parent-user-login=ogprodigy;turbo=0;user-type=;reply-parent-msg-id=a504ba7e-d991-45d0-ab2f-c3045c6ae7b6;reply-thread-parent-user-login=romeogiggletoess;returning-chatter=0;display-name=RomeoGiggleToess;badge-info=;reply-thread-parent-user-id=86336791;rm-received-ts=1723702053240 :romeogiggletoess!romeogiggletoess@romeogiggletoess.tmi.twitch.tv PRIVMSG #pajlada :@OGprodigy klassiker";
      IrcMessageRef::parse(data).unwrap();
    }
  }
}
