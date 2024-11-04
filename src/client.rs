//! ## Twitch IRC Client
//!
//! This is the main interface for interacting with Twitch IRC.
//! The entrypoint to this module is the [`Client`].
//!
//! The simplest way to get started is using [`Client::anonymous`],
//! which will connect to Twitch IRC anonymously.
//!
//! ```rust
//! # async fn run() -> anyhow::Result<()> {
//! let client = tmi::Client::anonymous().await?;
//! # Ok(())
//! # }
//! ```
//!
//! If you wish to be able to send messages, you have to generate an oauth2 token,
//! and provide it to the client:
//!
//! ```rust
//! # async fn run() -> anyhow::Result<()> {
//! let credentials = tmi::Credentials::new("your_username", "oauth:your_token");
//! let client = tmi::Client::builder().credentials(credentials).connect().await?;
//! # Ok(())
//! # }
//! ```
//!
//! and then use [`Client::builder`] followed by [`ClientBuilder::credentials`].
//!
//! Generating an oauth2 token is out of scope for this library.
//! Head over to the [official documentation](https://dev.twitch.tv/docs/irc/authenticate-bot/#getting-an-access-token)
//! to see how you can generate one. [twitch_oauth2](https://crates.io/crates/twitch_oauth2) may be used to automate most of it.
//!
//! ⚠ Note: [`Client`] is a fairly low-level interface! It does not automatically handle:
//! - Rate limiting (both for JOINs and PRIVMSGs)
//! - Same message bypass
//! - `RECONNECT` commands
//! - Rejoining channels
//! - Latency measurement
//!
//! What it _does_ provide is:
//! - Opening a TCP connection (with TLS) to Twitch.
//! - Performing the handshake (authentication, capability negotiation)
//! - Reconnect with backoff
//! - A polling interface for receiving messages
//! - Sending commands (PRIVMSG, JOIN, PONG, etc.)

#[macro_use]
mod macros;

pub mod conn;
pub mod read;
pub mod util;
pub mod write;

use self::conn::{OpenStreamError, TlsConfig, TlsConfigError};
use self::read::{ReadStream, RecvError};
use self::write::WriteStream;
use crate::irc::Command;
use crate::IrcMessage;
use futures_util::StreamExt;
use rand::{thread_rng, Rng};
use std::fmt::{Display, Write};
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_rustls::rustls::client::InvalidDnsNameError;
use tokio_rustls::rustls::ServerName;
use tokio_stream::wrappers::LinesStream;
use util::Timeout;

/// The default timeout used when connecting to Twitch IRC.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

fn justinfan() -> String {
  format!("justinfan{}", thread_rng().gen_range(10000..99999))
}

/// Reconnect backoff configuration.
#[derive(Clone, Copy, Debug)]
pub struct Backoff {
  /// The maximum number of reconnect attempts to make.
  pub max_tries: Option<u64>,

  /// Before attempting the first reconnect, the client will wait this long.
  pub initial_delay: Duration,

  /// After each failed reconnect attempt, the delay will be multiplied by this value.
  pub delay_multiplier: u32,

  /// The maximum delay to wait inbetween connection attempts.
  pub max_delay: Duration,
}

impl Default for Backoff {
  fn default() -> Self {
    Self {
      max_tries: Some(8),
      initial_delay: Duration::from_secs(1),
      delay_multiplier: 3,
      max_delay: Duration::from_secs(12),
    }
  }
}

/// Client configuration.
#[derive(Clone)]
pub struct Config {
  /// `token` should be a User Access Token.
  ///
  /// You can generate one by following the instructions on [Authorization Code Grant Flow](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#authorization-code-grant-flow).
  ///
  /// Make sure the token is valid before attempting to use it, and refresh it or generate a new one if it expires.
  ///
  /// [twitch_oauth2](https://crates.io/crates/twitch_oauth2) can help automate most of this.
  pub token: Option<String>,

  /// Connect and reconnect timeout.
  pub timeout: Duration,

  /// Reconnect backoff.
  pub backoff: Backoff,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      token: None,
      timeout: DEFAULT_TIMEOUT,
      backoff: Default::default(),
    }
  }
}

impl Config {
  /// Set the OAuth token.
  pub fn token(mut self, token: Option<impl Into<String>>) -> Self {
    self.token = token.map(|t| t.into());
    self
  }

  /// Set the timeout used on various operations, such as connecting and reconnecting.
  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// Set the backoff settings used when reconnecting.
  pub fn backoff(mut self, backoff: Backoff) -> Self {
    self.backoff = backoff;
    self
  }
}

impl std::fmt::Debug for Config {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Config")
      .field("token", &"<redacted>")
      .field("timeout", &self.timeout)
      .field("backoff", &self.backoff)
      .finish_non_exhaustive()
  }
}

/// Twitch IRC client.
///
/// This is the main interface for interacting with Twitch IRC.
///
/// This interface provides:
/// - Connection handshake
/// - Reconnect with backoff
/// - Receiving and sending messages
///
/// It is a low-level interface, which means it does not automatically handle:
/// - Rate limiting
/// - Same message bypass
/// - Reconnects / rejoining channels
/// - Latency measurement
pub struct Client {
  reader: ReadStream,
  writer: WriteStream,

  scratch: String,
  tls: TlsConfig,
  config: Config,
}

impl Client {
  /// Attempts to connect with the provided `config` and `timeout`.
  pub async fn connect(config: Config) -> Result<Client, ConnectError> {
    trace!("connecting");
    let tls = TlsConfig::load(ServerName::try_from(conn::HOST)?)?;
    trace!("opening connection to twitch");
    let timeout = config.timeout;
    let stream = conn::open(tls.clone()).timeout(timeout).await??;
    let (reader, writer) = split(stream);
    let mut client = Client {
      reader,
      writer,
      scratch: String::with_capacity(1024),
      tls,
      config,
    };
    client.handshake().timeout(timeout).await??;
    Ok(client)
  }

  /// Attempt to reconnect to Twitch IRC.
  pub async fn reconnect(&mut self) -> Result<(), ReconnectError> {
    trace!("reconnecting");

    let backoff = self.config.backoff;
    let timeout = self.config.timeout;
    let mut tries = backoff.max_tries;
    let mut delay = backoff.initial_delay;
    let mut cause = ConnectError::Timeout;
    while matches!(tries, None | Some(1..)) {
      tokio::time::sleep(delay).await;
      if let Some(tries) = &mut tries {
        *tries -= 1;
      }
      delay = std::cmp::min(backoff.max_delay, delay * backoff.delay_multiplier);

      trace!("opening connection to twitch");
      let stream = match conn::open(self.tls.clone()).timeout(timeout).await? {
        Ok(stream) => stream,
        Err(e @ OpenStreamError::Io(_)) => {
          cause = e.into();
          continue;
        }
      };

      (self.reader, self.writer) = split(stream);

      if let Err(e) = self.handshake().timeout(timeout).await? {
        if e.should_retry() {
          cause = e;
          continue;
        }
        return Err(e.into());
      };

      return Ok(());
    }

    Err(ReconnectError { cause })
  }

  async fn handshake(&mut self) -> Result<(), ConnectError> {
    trace!("performing handshake");

    const CAP: &str = "twitch.tv/commands twitch.tv/tags twitch.tv/membership";
    trace!("CAP REQ {CAP:?}; PASS <redacted>");
    write!(&mut self.scratch, "CAP REQ :{CAP}\r\n").unwrap();

    match &self.config.token {
      Some(token) => {
        let oauth = if token.starts_with("oauth:") {
          ""
        } else {
          "oauth:"
        };
        write!(&mut self.scratch, "PASS {oauth}{token}\r\n").unwrap();
      }
      None => {
        write!(&mut self.scratch, "PASS just_a_lil_guy\r\n").unwrap();
        write!(&mut self.scratch, "NICK {}\r\n", justinfan()).unwrap();
      }
    }

    self.writer.write_all(self.scratch.as_bytes()).await?;
    self.writer.flush().await?;
    self.scratch.clear();

    trace!("waiting for CAP * ACK");
    let message = self.recv().timeout(Duration::from_secs(5)).await??;
    trace!(?message, "received message");

    match message.command() {
      Command::Capability => {
        if message.params().is_some_and(|v| v.starts_with("* ACK")) {
          trace!("received CAP * ACK")
        } else {
          return Err(ConnectError::Auth);
        }
      }
      _ => {
        trace!("unexpected message");
        return Err(ConnectError::Welcome(Box::new(message)));
      }
    }

    trace!("waiting for NOTICE 001");
    let message = self.recv().timeout(Duration::from_secs(5)).await??;
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
          return Err(ConnectError::Auth);
        }

        trace!("unrecognized error");
        return Err(ConnectError::Notice(Box::new(message)));
      }
      _ => {
        trace!("first message not recognized");
        return Err(ConnectError::Welcome(Box::new(message)));
      }
    }

    Ok(())
  }
}

impl Client {
  #[inline]
  pub fn config(&self) -> &Config {
    &self.config
  }
}

fn split(stream: conn::Stream) -> (ReadStream, WriteStream) {
  let (reader, writer) = tokio::io::split(stream);

  (
    LinesStream::new(BufReader::new(reader).lines()).fuse(),
    writer,
  )
}

/// An error which occurred while attempting to reconnect to Twitch IRC.
#[derive(Debug)]
pub struct ReconnectError {
  /// The last encountered error.
  pub cause: ConnectError,
}

impl Display for ReconnectError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "all reconnect attempts failed. last error was: {}",
      self.cause
    )
  }
}

impl std::error::Error for ReconnectError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.cause)
  }

  fn cause(&self) -> Option<&dyn std::error::Error> {
    self.source()
  }
}

impl<T: Into<ConnectError>> From<T> for ReconnectError {
  fn from(cause: T) -> Self {
    Self {
      cause: cause.into(),
    }
  }
}

/// An error which occurred while attempting to connect to Twitch IRC.
#[derive(Debug)]
pub enum ConnectError {
  /// Failed to read from the stream.
  Read(RecvError),

  /// Failed to perform an IO operation on the stream.
  Io(io::Error),

  /// Failed to query DNS.
  Dns(InvalidDnsNameError),

  /// Failed to establish TLS connection.
  Tls(TlsConfigError),

  /// Failed to open a connection.
  Open(OpenStreamError),

  /// Connection timed out.
  Timeout,

  /// Connection received invalid welcome message.
  Welcome(Box<IrcMessage>),

  /// Failed to connect because of invalid credentials.
  Auth,

  /// Twitch sent a notice that we didn't expect during the handshake.
  Notice(Box<IrcMessage>),
}

impl ConnectError {
  fn should_retry(&self) -> bool {
    matches!(self, Self::Open(OpenStreamError::Io(_)) | Self::Io(_))
  }
}

impl From<RecvError> for ConnectError {
  fn from(value: RecvError) -> Self {
    Self::Read(value)
  }
}

impl From<io::Error> for ConnectError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<InvalidDnsNameError> for ConnectError {
  fn from(value: InvalidDnsNameError) -> Self {
    Self::Dns(value)
  }
}

impl From<TlsConfigError> for ConnectError {
  fn from(value: TlsConfigError) -> Self {
    Self::Tls(value)
  }
}

impl From<OpenStreamError> for ConnectError {
  fn from(value: OpenStreamError) -> Self {
    Self::Open(value)
  }
}

impl From<tokio::time::error::Elapsed> for ConnectError {
  fn from(_: tokio::time::error::Elapsed) -> Self {
    Self::Timeout
  }
}

impl Display for ConnectError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ConnectError::Read(e) => write!(f, "failed to connect: {e}"),
      ConnectError::Io(e) => write!(f, "failed to connect: {e}"),
      ConnectError::Dns(e) => write!(f, "failed to connect: {e}"),
      ConnectError::Tls(e) => write!(f, "failed to connect: {e}"),
      ConnectError::Open(e) => write!(f, "failed to connect: {e}"),
      ConnectError::Timeout => write!(f, "failed to connect: connection timed out"),
      ConnectError::Welcome(msg) => write!(
        f,
        "failed to connect: expected `NOTICE` or `001` as first message, instead received: {msg:?}"
      ),
      ConnectError::Auth => write!(f, "failed to connect: invalid credentials"),
      ConnectError::Notice(msg) => write!(
        f,
        "failed to connect: received unrecognized notice: {msg:?}"
      ),
    }
  }
}

impl std::error::Error for ConnectError {}

static_assert_send!(Client);
static_assert_sync!(Client);
