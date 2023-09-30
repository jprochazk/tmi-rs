use super::{conn, Client};
use crate::common::{Channel, InvalidChannelName};
use std::fmt::Display;
use tokio::io;
use tokio::io::{AsyncWriteExt, WriteHalf};

pub type WriteStream = WriteHalf<conn::Stream>;

impl Client {
  /// Send a raw string through the TCP socket.
  ///
  /// ⚠ This call is not rate limited in any way.
  ///
  /// ⚠ The string MUST be terminated by `\r\n`.
  pub async fn send<'a, S>(&mut self, s: S) -> Result<(), WriteError>
  where
    S: TryInto<RawMessage<'a>, Error = InvalidMessage> + 'a,
  {
    let RawMessage { data } = s.try_into()?;
    tracing::trace!(data, "sending message");
    self.writer.write_all(data.as_bytes()).await?;
    Ok(())
  }

  pub async fn join<'a, S>(&mut self, channel: S) -> Result<(), WriteError>
  where
    S: TryInto<Channel<'a>, Error = InvalidChannelName> + 'a,
  {
    use std::fmt::Write;

    with_scratch!(self, |f| {
      let channel = channel.try_into()?;
      let _ = write!(f, "JOIN {channel}\r\n");

      tracing::trace!(data = f, "sending message");
      self.send(f.as_str()).await?;
    });

    Ok(())
  }

  /// Send a `JOIN` command.
  ///
  /// ⚠ This call is not rate limited in any way.
  ///
  /// ⚠ Each channel in `channels` MUST be a valid channel name
  /// prefixed by `#`.
  pub async fn join_all<'a, I, S>(&mut self, channels: I) -> Result<(), WriteError>
  where
    I: IntoIterator<Item = S>,
    S: TryInto<Channel<'a>, Error = InvalidChannelName> + 'a,
  {
    use std::fmt::Write;

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

      tracing::trace!(data = f, "sending message");
      self.send(f.as_str()).await?;
    });

    Ok(())
  }
}

#[derive(Debug)]
pub enum WriteError {
  Io(io::Error),
  StreamClosed,
  InvalidMessage(InvalidMessage),
  InvalidChannelName(InvalidChannelName),
}

impl From<io::Error> for WriteError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<InvalidMessage> for WriteError {
  fn from(value: InvalidMessage) -> Self {
    Self::InvalidMessage(value)
  }
}

impl From<InvalidChannelName> for WriteError {
  fn from(value: InvalidChannelName) -> Self {
    Self::InvalidChannelName(value)
  }
}

impl Display for WriteError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      WriteError::Io(e) => write!(f, "failed to write message: {e}"),
      WriteError::StreamClosed => write!(f, "failed to write message: stream closed"),
      WriteError::InvalidMessage(inner) => write!(
        f,
        "failed to write message: message was incorrectly formatted, {inner}"
      ),
      WriteError::InvalidChannelName(inner) => write!(
        f,
        "failed to write message: message was incorrectly formatted, {inner}"
      ),
    }
  }
}

impl std::error::Error for WriteError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SameMessageBypass {
  append: bool,
}

impl SameMessageBypass {
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
