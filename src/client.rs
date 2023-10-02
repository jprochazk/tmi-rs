//! ## Twitch IRC Client
//!
//! This is the main interface for interacting with Twitch IRC.
//! The entrypoint to this module is the [`Client`].
//!
//! The simplest way to get started is using [`Client::connect`],
//! which will connect to Twitch IRC anonymously,
//! followed by joining some channels using [`Client::join`].
//!
//! If you wish to be able to send messages, you have to generate an oauth2 token,
//! and then supply the [`Credentials`] to the client via [`Client::connect_with`],
//! or [`Client::builder`] followed by [`ClientBuilder::credentials`].
//!
//! Generating an oauth2 token is out of scope for this library.
//! Head over to the [official documentation](https://dev.twitch.tv/docs/irc/authenticate-bot/#getting-an-access-token)
//! to see how you can generate one.
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

use self::conn::TlsConfig;
use self::conn::{OpenStreamError, TlsConfigError};
use self::read::ReadStream;
use self::read::RecvError;
use self::write::WriteStream;
use crate::irc::Command;
use crate::IrcMessage;
use futures_util::StreamExt;
use rand::{thread_rng, Rng};
use std::fmt::{Display, Write};
use std::future::Future;
use std::io;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_rustls::rustls::client::InvalidDnsNameError;
use tokio_rustls::rustls::ServerName;
use tokio_stream::wrappers::LinesStream;
use util::Timeout;

/// Credentials used to authenticate to Twitch IRC.
///
/// The [`Default`] impl uses [`Credentials::anon`].
#[derive(Clone)]
pub struct Credentials {
  /// The _login_ of the user.
  pub nick: String,

  /// The oauth2 token.
  pub pass: String,
}

impl Credentials {
  const ANON_RANGE: std::ops::Range<u32> = 10000..99999;

  /// Instantiate credentials from a `nick` and `pass`.
  ///
  /// This does nothing except make it a little more convenient
  /// to construct `Credentials` from various string types.
  pub fn new(nick: impl ToString, pass: impl ToString) -> Self {
    Self {
      nick: nick.to_string(),
      pass: pass.to_string(),
    }
  }

  /// An anonymous login.
  ///
  /// Twitch allows logging in using any username in the form `justinfan?????`
  /// where `?` is any digit. For example, `justinfan11824` is a valid username.
  ///
  /// If you login anonymously, you won't be able to send messages, but you
  /// will still be able to read them, including all the usual tags,
  /// membership commands, etc.
  pub fn anon() -> Self {
    Self {
      pass: "just_a_lil_guy".into(),
      nick: format!("justinfan{}", thread_rng().gen_range(Self::ANON_RANGE)),
    }
  }

  pub fn is_anon(&self) -> bool {
    let Some(digits) = self.nick.strip_prefix("justinfan") else {
      return false;
    };
    let Some(digits) = digits.parse::<u32>().ok() else {
      return false;
    };
    Self::ANON_RANGE.contains(&digits)
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

/// Client configuration.
///
/// At the moment this only holds credentials.
#[derive(Clone, Debug, Default)]
pub struct Config {
  /// Credentials to use when logging in to Twitch IRC.
  pub credentials: Credentials,
}

impl Config {
  /// Instantiate a config from some `credentials`.
  pub fn new(credentials: Credentials) -> Self {
    Self { credentials }
  }
}

/// Builder for a [`Client`].
pub struct ClientBuilder {
  config: Config,
}

/// The default timeout used when connecting to Twitch IRC.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Reconnect backoff configuration.
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

/// The default reconnect backoff.
pub const DEFAULT_BACKOFF: Backoff = Backoff {
  max_tries: Some(8),
  initial_delay: Duration::from_secs(1),
  delay_multiplier: 3,
  max_delay: Duration::from_secs(12),
};

impl ClientBuilder {
  /// Set the credentials.
  pub fn credentials(mut self, credentials: Credentials) -> Self {
    self.config.credentials = credentials;
    self
  }

  /// Attempts to connect to Twitch IRC using this configuration.
  ///
  /// This uses the [`DEFAULT_TIMEOUT`].
  pub fn connect(self) -> impl Future<Output = Result<Client, ConnectError>> {
    Client::connect_with(self.config, DEFAULT_TIMEOUT)
  }

  /// Attempts to connect to Twitch IRC using this configuration.
  ///
  /// Uses the provided `timeout`.
  pub fn connect_with_timeout(
    self,
    timeout: Duration,
  ) -> impl Future<Output = Result<Client, ConnectError>> {
    Client::connect_with(self.config, timeout)
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
  /// The [`ClientBuilder`] provides a builder for setting up the client configuration.
  pub fn builder() -> ClientBuilder {
    ClientBuilder {
      config: Default::default(),
    }
  }

  /// Attemps to connect with the default configuration.
  ///
  /// This uses the [`DEFAULT_TIMEOUT`].
  ///
  /// This connection is anonymous, which means you can't send messages.
  pub fn connect() -> impl Future<Output = Result<Client, ConnectError>> {
    Self::connect_with(Config::default(), DEFAULT_TIMEOUT)
  }

  /// Attempts to connect with the provided `config` and `timeout`.
  pub async fn connect_with(config: Config, timeout: Duration) -> Result<Client, ConnectError> {
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
      config,
    };
    chat.handshake().timeout(timeout).await??;
    Ok(chat)
  }

  /// Attempt to reconnect to Twitch IRC.
  ///
  /// This uses the [`DEFAULT_BACKOFF`] and [`DEFAULT_TIMEOUT`].
  pub fn reconnect(&mut self) -> impl Future<Output = Result<(), ReconnectError>> + '_ {
    self.reconnect_with(DEFAULT_BACKOFF, DEFAULT_TIMEOUT)
  }

  /// Attempt to reconnect to Twitch IRC.
  ///
  /// This uses the provided `backoff` and `timeout`.
  pub async fn reconnect_with(
    &mut self,
    backoff: Backoff,
    timeout: Duration,
  ) -> Result<(), ReconnectError> {
    trace!("reconnecting");

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
        } else {
          return Err(e.into());
        }
      };

      return Ok(());
    }

    Err(ReconnectError { cause })
  }

  async fn handshake(&mut self) -> Result<(), ConnectError> {
    trace!("performing handshake");

    let credentials = &self.config.credentials;
    const CAP: &str = "twitch.tv/commands twitch.tv/tags twitch.tv/membership";
    trace!("CAP REQ {CAP:?}; NICK {:?}; PASS ***", credentials.nick);
    write!(&mut self.scratch, "CAP REQ :{CAP}\r\n").unwrap();
    write!(&mut self.scratch, "PASS {}\r\n", credentials.pass).unwrap();
    write!(&mut self.scratch, "NICK {}\r\n", credentials.nick).unwrap();
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
        return Err(ConnectError::Welcome(message));
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
        } else {
          trace!("unrecognized error");
          return Err(ConnectError::Notice(message));
        }
      }
      _ => {
        trace!("first message not recognized");
        return Err(ConnectError::Welcome(message));
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

  #[inline]
  pub fn credentials(&self) -> &Credentials {
    &self.config.credentials
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
  Welcome(IrcMessage),

  /// Failed to connect because of invalid credentials.
  Auth,

  /// Twitch sent a notice that we didn't expect during the handshake.
  Notice(IrcMessage),
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
