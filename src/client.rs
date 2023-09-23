mod channel;
mod conn;
mod ratelimit;
mod read;
mod util;
mod write;

use self::channel::Channel;
use self::conn::TlsConfig;
use self::read::{ReadError, ReadStream};
use self::write::WriteStream;
use crate::msg::Command;
use futures_util::StreamExt;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::future::Future;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_rustls::rustls::client::InvalidDnsNameError;
use tokio_rustls::rustls::ServerName;
use tokio_stream::wrappers::LinesStream;
use util::Timeout;

pub use self::conn::{OpenStreamError, TlsConfigError};

#[derive(Clone)]
pub struct Credentials {
  pub nick: String,
  pub pass: String,
}

impl Credentials {
  pub fn new(nick: impl ToString, pass: impl ToString) -> Self {
    Self {
      nick: nick.to_string(),
      pass: pass.to_string(),
    }
  }

  pub fn anon() -> Self {
    Self {
      pass: "just_a_lil_guy".into(),
      nick: format!("justinfan{}", thread_rng().gen_range(10000u32..99999u32)),
    }
  }
}

impl Default for Credentials {
  fn default() -> Self {
    Self::anon()
  }
}

impl std::fmt::Debug for Credentials {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Credentials")
      .field("nick", &self.nick)
      .finish_non_exhaustive()
  }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
  pub credentials: Credentials,
}

impl Config {
  pub fn new(credentials: Credentials) -> Self {
    Self { credentials }
  }
}

pub struct ClientBuilder {
  config: Config,
}

impl ClientBuilder {
  pub fn credentials(mut self, credentials: Credentials) -> Self {
    self.config.credentials = credentials;
    self
  }

  pub fn connect(self, timeout: Duration) -> impl Future<Output = Result<Client, ConnectionError>> {
    Client::connect(self.config, timeout)
  }
}

pub struct Client {
  reader: ReadStream,
  writer: WriteStream,

  scratch: String,
  tls: TlsConfig,
  credentials: Credentials,

  channels: HashMap<String, Channel>,
}

impl Client {
  pub fn builder() -> ClientBuilder {
    ClientBuilder {
      config: Default::default(),
    }
  }

  pub async fn connect(config: Config, timeout: Duration) -> Result<Client, ConnectionError> {
    trace!("connecting");
    let tls = TlsConfig::load(ServerName::try_from(conn::HOST)?)?;
    trace!("opening connection to twitch");
    let stream = conn::open(tls.clone()).timeout(timeout).await??;
    let (reader, writer) = split(stream);
    let mut chat = Client {
      reader,
      writer,
      scratch: String::with_capacity(1024),
      tls,
      credentials: config.credentials,
      channels: HashMap::new(),
    };
    chat.handshake().timeout(timeout).await??;
    Ok(chat)
  }

  pub async fn reconnect(&mut self, timeout: Duration) -> Result<(), ConnectionError> {
    trace!("reconnecting");

    let mut tries = 10;
    let mut delay = Duration::from_secs(3);

    while tries != 0 {
      tokio::time::sleep(delay).await;
      tries -= 1;
      delay *= 3;

      trace!("opening connection to twitch");
      let stream = match conn::open(self.tls.clone()).timeout(timeout).await? {
        Ok(stream) => stream,
        Err(OpenStreamError::Io(_)) => continue,
      };

      (self.reader, self.writer) = split(stream);

      if let Err(e) = self.handshake().timeout(timeout).await? {
        if e.should_retry() {
          continue;
        } else {
          return Err(e);
        }
      };

      return Ok(());
    }

    Err(ConnectionError::Reconnect)
  }

  async fn handshake(&mut self) -> Result<(), ConnectionError> {
    trace!("performing handshake");

    const CAP: &str = "twitch.tv/commands twitch.tv/tags";
    trace!("CAP REQ {CAP}; NICK {}; PASS ***", self.credentials.nick);

    write!(&mut self.scratch, "CAP REQ :{CAP}\r\n").unwrap();
    write!(&mut self.scratch, "NICK {}\r\n", self.credentials.nick).unwrap();
    write!(&mut self.scratch, "PASS {}\r\n", self.credentials.pass).unwrap();

    self.writer.write_all(self.scratch.as_bytes()).await?;
    self.writer.flush().await?;
    self.scratch.clear();

    trace!("waiting for CAP * ACK");
    let message = self.message().timeout(Duration::from_secs(5)).await??;
    trace!(?message, "received message");

    match message.command() {
      Command::Capability => {
        if message.params().is_some_and(|v| v.starts_with("* ACK")) {
          trace!("received CAP * ACK")
        } else {
          return Err(ConnectionError::Auth);
        }
      }
      _ => {
        trace!("unexpected message");
        return Err(ConnectionError::Welcome(message));
      }
    }

    trace!("waiting for NOTICE 001");
    let message = self.message().timeout(Duration::from_secs(5)).await??;
    trace!(?message, "received message");

    match message.command() {
      Command::RplWelcome => {
        trace!("connected");
      }
      Command::Notice => {
        if message
          .params()
          .map(|v| v.contains("authentication failed"))
          .unwrap_or(false)
        {
          trace!("invalid credentials");
          return Err(ConnectionError::Auth);
        } else {
          trace!("unrecognized error");
          return Err(ConnectionError::Notice(message));
        }
      }
      _ => {
        trace!("first message not recognized");
        return Err(ConnectionError::Welcome(message));
      }
    }

    Ok(())
  }
}

fn split(stream: conn::Stream) -> (ReadStream, WriteStream) {
  let (reader, writer) = tokio::io::split(stream);

  (
    LinesStream::new(BufReader::new(reader).lines()).fuse(),
    writer,
  )
}

#[derive(Debug)]
pub enum ConnectionError {
  /// Failed to read from the stream.
  Read(ReadError),

  /// Failed to perform an IO operation on the stream.
  Io(io::Error),

  /// Failed to query DNS.
  Dns(InvalidDnsNameError),

  /// Failed to establish TLS connection.
  Tls(TlsConfigError),

  /// Failed to open a connection.
  Open(OpenStreamError),

  /// Connection timed out.
  Timeout(tokio::time::error::Elapsed),

  /// Connection received invalid welcome message.
  Welcome(String),

  /// Failed to connect because of invalid credentials.
  Auth,

  /// Twitch sent a notice that we didn't expect during the handshake.
  Notice(String),

  /// Failed to reconnect.
  Reconnect,
}

impl ConnectionError {
  fn should_retry(&self) -> bool {
    matches!(self, Self::Open(OpenStreamError::Io(_)) | Self::Io(_))
  }
}

impl From<ReadError> for ConnectionError {
  fn from(value: ReadError) -> Self {
    Self::Read(value)
  }
}

impl From<io::Error> for ConnectionError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<InvalidDnsNameError> for ConnectionError {
  fn from(value: InvalidDnsNameError) -> Self {
    Self::Dns(value)
  }
}

impl From<TlsConfigError> for ConnectionError {
  fn from(value: TlsConfigError) -> Self {
    Self::Tls(value)
  }
}

impl From<OpenStreamError> for ConnectionError {
  fn from(value: OpenStreamError) -> Self {
    Self::Open(value)
  }
}

impl From<tokio::time::error::Elapsed> for ConnectionError {
  fn from(value: tokio::time::error::Elapsed) -> Self {
    Self::Timeout(value)
  }
}

impl Display for ConnectionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ConnectionError::Read(e) => write!(f, "failed to connect: {e}"),
      ConnectionError::Io(e) => write!(f, "failed to connect: {e}"),
      ConnectionError::Dns(e) => write!(f, "failed to connect: {e}"),
      ConnectionError::Tls(e) => write!(f, "failed to connect: {e}"),
      ConnectionError::Open(e) => write!(f, "failed to connect: {e}"),
      ConnectionError::Timeout(e) => write!(f, "failed to connect: connection timed out, {e}"),
      ConnectionError::Welcome(msg) => write!(
        f,
        "failed to connect: expected `NOTICE` or `001` as first message, instead received: {msg:?}"
      ),
      ConnectionError::Auth => write!(f, "failed to connect: invalid credentials"),
      ConnectionError::Notice(msg) => write!(
        f,
        "failed to connect: received unrecognized notice: {msg:?}"
      ),
      ConnectionError::Reconnect => write!(f, "failed to connect: reconnect attempt failed"),
    }
  }
}

impl std::error::Error for ConnectionError {}
