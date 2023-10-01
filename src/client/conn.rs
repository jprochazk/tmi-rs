use std::fmt::Display;
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerName};
use tokio_rustls::{rustls, TlsConnector};

pub const HOST: &str = "irc.chat.twitch.tv";
pub const PORT: u16 = 6697;

pub type Stream = TlsStream<TcpStream>;

pub async fn open(config: TlsConfig) -> Result<Stream, OpenStreamError> {
  trace!(?config, "opening tls stream to twitch");
  Ok(
    TlsConnector::from(config.client())
      .connect(
        config.server_name(),
        TcpStream::connect((HOST, PORT)).await?,
      )
      .await?,
  )
}

/// Failed to open a TLS stream.
#[derive(Debug)]
pub enum OpenStreamError {
  /// The underlying I/O operation failed.
  Io(io::Error),
}

impl From<io::Error> for OpenStreamError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl Display for OpenStreamError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      OpenStreamError::Io(e) => write!(f, "failed to open tls stream: {e}"),
    }
  }
}

impl std::error::Error for OpenStreamError {}

#[derive(Debug, Clone)]
pub struct TlsConfig {
  config: Arc<ClientConfig>,
  server_name: ServerName,
}

impl TlsConfig {
  pub fn load(server_name: ServerName) -> Result<Self, TlsConfigError> {
    trace!("loading native certificates");
    let mut root_store = RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs()?;
    for cert in native_certs {
      root_store.add(&rustls::Certificate(cert.0))?;
    }
    let config = rustls::ClientConfig::builder()
      .with_safe_defaults()
      .with_root_certificates(root_store)
      .with_no_client_auth();
    Ok(Self {
      config: Arc::new(config),
      server_name,
    })
  }

  pub fn client(&self) -> Arc<ClientConfig> {
    self.config.clone()
  }

  pub fn server_name(&self) -> ServerName {
    self.server_name.clone()
  }
}

/// Failed to load the TLS config.
#[derive(Debug)]
pub enum TlsConfigError {
  /// The underlying I/O operation failed.
  Io(io::Error),
  /// Failed to load certificates.
  Tls(rustls::Error),
}

impl From<io::Error> for TlsConfigError {
  fn from(value: io::Error) -> Self {
    Self::Io(value)
  }
}

impl From<rustls::Error> for TlsConfigError {
  fn from(value: rustls::Error) -> Self {
    Self::Tls(value)
  }
}

impl Display for TlsConfigError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TlsConfigError::Io(e) => write!(f, "tls config error: {e}"),
      TlsConfigError::Tls(e) => write!(f, "tls config error: {e}"),
    }
  }
}

impl std::error::Error for TlsConfigError {}
