use super::{conn, Client};
use std::fmt::Display;
use tokio::io;
use tokio::io::{AsyncWriteExt, WriteHalf};

pub type WriteStream = WriteHalf<conn::Stream>;

impl Client {
  pub async fn send(&mut self, s: &str) -> Result<(), WriteError> {
    self.writer.write_all(s.as_bytes()).await?;
    Ok(())
  }
}

#[derive(Debug)]
pub enum WriteError {
  Io(io::Error),
  StreamClosed,
}

impl From<io::Error> for WriteError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl Display for WriteError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      WriteError::Io(e) => write!(f, "failed to write message: {e}"),
      WriteError::StreamClosed => write!(f, "failed to write message: stream closed"),
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
