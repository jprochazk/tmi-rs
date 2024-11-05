use super::{conn, Client};
use crate::irc::IrcMessage;
use futures_util::stream::Fuse;
use std::fmt::Display;
use tokio::io;
use tokio::io::{BufReader, ReadHalf};
use tokio_stream::wrappers::LinesStream;
use tokio_stream::StreamExt;

pub type ReadStream = Fuse<LinesStream<BufReader<ReadHalf<conn::Stream>>>>;

impl Client {
  /// Read a single [`IrcMessage`] from the underlying stream.
  pub async fn recv(&mut self) -> Result<IrcMessage, RecvError> {
    if let Some(message) = self.reader.next().await {
      let message = message?;
      Ok(IrcMessage::parse(message).map_err(RecvError::Parse)?)
    } else {
      Err(RecvError::StreamClosed)
    }
  }
}

/// Failed to receive a message.
#[derive(Debug)]
pub enum RecvError {
  /// The underlying I/O operation failed.
  Io(io::Error),

  /// Failed to parse the message.
  Parse(String),

  /// The stream was closed.
  StreamClosed,
}

impl RecvError {
  /// Returns `true` if this `recv` failed due to a disconnect of some kind.
  pub fn is_disconnect(&self) -> bool {
    match self {
      RecvError::StreamClosed => true,
      RecvError::Io(e)
        if matches!(
          e.kind(),
          io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionAborted | io::ErrorKind::TimedOut
        ) =>
      {
        true
      }
      _ => false,
    }
  }
}

impl From<io::Error> for RecvError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl Display for RecvError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RecvError::Io(e) => write!(f, "failed to read message: {e}"),
      RecvError::Parse(s) => write!(f, "failed to read message: invalid message `{s}`"),
      RecvError::StreamClosed => write!(f, "failed to read message: stream closed"),
    }
  }
}

impl std::error::Error for RecvError {}
