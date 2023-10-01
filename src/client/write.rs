use super::{conn, Client};
use crate::common::JoinIter;
use crate::common::{Channel, InvalidChannelName};
use std::fmt::Display;
use tokio::io;
use tokio::io::{AsyncWriteExt, WriteHalf};

pub type WriteStream = WriteHalf<conn::Stream>;

pub struct Privmsg<'c, 'a> {
  client: &'c mut Client,
  channel: Channel<'a>,
  text: &'a str,
  reply_parent_msg_id: Option<&'a str>,
  client_nonce: Option<&'a str>,
}

struct Tag<'a> {
  key: &'a str,
  value: &'a str,
}

impl<'a> std::fmt::Display for Tag<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Self { key, value } = self;
    // TODO: handle escaping
    write!(f, "{key}={value}")
  }
}

impl<'c, 'a> Privmsg<'c, 'a> {
  pub async fn send(self) -> Result<(), SendError> {
    let Self {
      client,
      channel,
      text,
      reply_parent_msg_id,
      client_nonce,
    } = self;

    with_scratch!(client, |f| {
      let has_tags = reply_parent_msg_id.is_some() || client_nonce.is_some();
      if has_tags {
        let reply_parent_msg_id = reply_parent_msg_id.map(|value| Tag {
          key: "reply-parent-msg-id",
          value,
        });
        let client_nonce = client_nonce.map(|value| Tag {
          key: "client-nonce",
          value,
        });
        let tags = reply_parent_msg_id
          .iter()
          .chain(client_nonce.iter())
          .join(';');
        let _ = write!(f, "@{tags} ");
      }
      let _ = write!(f, "PRIVMSG {channel} :{text}\r\n");
      client.send_raw(f.as_str()).await
    })
  }
}

impl Client {
  /// Send a raw string through the TCP socket.
  ///
  /// ⚠ This call is not rate limited in any way.
  ///
  /// ⚠ The string MUST be terminated by `\r\n`.
  pub async fn send_raw<'a, S>(&mut self, s: S) -> Result<(), SendError>
  where
    S: TryInto<RawMessage<'a>, Error = InvalidMessage> + 'a,
  {
    let RawMessage { data } = s.try_into()?;
    trace!(data, "sending message");
    self.writer.write_all(data.as_bytes()).await?;
    Ok(())
  }

  /// Create a `privmsg` from a `channel` and `text`.
  ///
  /// The message may be sent using [`Privmsg::send`].
  ///
  /// You can specify additional properties using the builder methods on [`Privmsg`]:
  /// - [`Privmsg::reply_to`] to specify a `reply-parent-msg-id` tag, which makes this privmsg a reply to another message.
  /// - [`Privmsg::client_nonce`] to identify the message in the `Notice` which Twitch may send as a response to this message.
  pub fn privmsg<'this, 'a, C, S>(
    &'this mut self,
    channel: C,
    text: &'a S,
  ) -> Result<Privmsg<'this, 'a>, SendError>
  where
    C: TryInto<Channel<'a>, Error = InvalidChannelName>,
    S: ?Sized + AsRef<str>,
  {
    Ok(Privmsg {
      client: self,
      channel: channel.try_into()?,
      text: text.as_ref(),
      reply_parent_msg_id: None,
      client_nonce: None,
    })
  }

  /// Send a `PING` command with an optional `nonce` argument.
  pub async fn ping(&mut self, nonce: &str) -> Result<(), SendError> {
    with_scratch!(self, |f| {
      let _ = write!(f, "PING :{nonce}\r\n");
      self.send_raw(f.as_str()).await
    })
  }

  /// Send a `PONG` command in response to a `PING`.
  pub async fn pong(&mut self, ping: &crate::Ping<'_>) -> Result<(), SendError> {
    with_scratch!(self, |f| {
      if let Some(nonce) = ping.nonce() {
        let _ = write!(f, "PONG :{nonce}\r\n");
      } else {
        let _ = write!(f, "PONG\r\n");
      }
      self.send_raw(f.as_str()).await
    })
  }

  /// Send a `JOIN` command.
  ///
  /// ⚠ This call is not rate limited in any way.
  ///
  /// ⚠ `channel` MUST be a valid channel name prefixed by `#`.
  pub async fn join<'a, S>(&mut self, channel: S) -> Result<(), SendError>
  where
    S: TryInto<Channel<'a>, Error = InvalidChannelName> + 'a,
  {
    with_scratch!(self, |f| {
      let channel = channel.try_into()?;
      let _ = write!(f, "JOIN {channel}\r\n");
      self.send_raw(f.as_str()).await
    })
  }

  /// Send a `JOIN` command.
  ///
  /// ⚠ This call is not rate limited in any way.
  ///
  /// ⚠ Each channel in `channels` MUST be a valid channel name
  /// prefixed by `#`.
  pub async fn join_all<'a, I, S>(&mut self, channels: I) -> Result<(), SendError>
  where
    I: IntoIterator<Item = S>,
    S: TryInto<Channel<'a>, Error = InvalidChannelName> + 'a,
  {
    with_scratch!(self, |f| {
      let _ = f.write_str("JOIN ");
      let mut channels = channels.into_iter();
      if let Some(channel) = channels.next() {
        let channel = channel.try_into()?;
        let _ = write!(f, "{channel}");
      }
      for channel in channels {
        let channel = channel.try_into()?;
        let _ = write!(f, ",{channel}");
      }
      let _ = f.write_str("\r\n");
      self.send_raw(f.as_str()).await
    })
  }
}

/// Failed to send a message.
#[derive(Debug)]
pub enum SendError {
  /// The underlying I/O operation failed.
  Io(io::Error),

  /// The stream was closed.
  StreamClosed,

  /// Attempted to send an invalid message.
  InvalidMessage(InvalidMessage),

  /// Attempted to send a message to a channel with an invalid name.
  InvalidChannelName(InvalidChannelName),
}

impl From<io::Error> for SendError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<InvalidMessage> for SendError {
  fn from(value: InvalidMessage) -> Self {
    Self::InvalidMessage(value)
  }
}

impl From<InvalidChannelName> for SendError {
  fn from(value: InvalidChannelName) -> Self {
    Self::InvalidChannelName(value)
  }
}

impl Display for SendError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SendError::Io(e) => write!(f, "failed to write message: {e}"),
      SendError::StreamClosed => write!(f, "failed to write message: stream closed"),
      SendError::InvalidMessage(inner) => write!(
        f,
        "failed to write message: message was incorrectly formatted, {inner}"
      ),
      SendError::InvalidChannelName(inner) => write!(
        f,
        "failed to write message: message was incorrectly formatted, {inner}"
      ),
    }
  }
}

impl std::error::Error for SendError {}

/// Bypass the same-message slow mode requirement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SameMessageBypass {
  append: bool,
}

impl SameMessageBypass {
  /// Get the current value.
  ///
  /// This is meant to be appended to the end of the message, before the `\r\n`.
  pub fn get(&mut self) -> &'static str {
    let out = match self.append {
      false => "",
      true => {
        concat!(" ", "⠀")
      }
    };
    self.append = !self.append;
    out
  }
}

#[allow(clippy::derivable_impls)]
impl Default for SameMessageBypass {
  fn default() -> Self {
    SameMessageBypass { append: false }
  }
}

/// An IRC message, terminated by `\r\n`.
pub struct RawMessage<'a> {
  data: &'a str,
}

#[derive(Debug)]
pub struct InvalidMessage;
impl std::fmt::Display for InvalidMessage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("not terminated by \"\\r\\n\"")
  }
}
impl std::error::Error for InvalidMessage {}

impl<'a> TryFrom<&'a str> for RawMessage<'a> {
  type Error = InvalidMessage;

  fn try_from(data: &'a str) -> Result<Self, Self::Error> {
    match data.ends_with("\r\n") {
      true => Ok(RawMessage { data }),
      false => Err(InvalidMessage),
    }
  }
}
