use futures::{
  stream::{SplitSink, SplitStream},
  StreamExt,
};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{
  connect_async,
  tungstenite::{self, Message},
  MaybeTlsStream, WebSocketStream,
};

#[derive(Error, Debug)]
pub enum Error {
  #[error("Connection to Twitch IRC server failed")]
  ConnectionFailed,
  #[error("Encountered an error on the underlying WebSocket connection: {0}")]
  WebSocketError(#[from] tungstenite::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub const URL: &str = "wss://pubsub-edge.twitch.tv";

pub async fn connect() -> Result<Connection> {
  let (stream, _) = connect_async(URL).await.map_err(|_| Error::ConnectionFailed)?;
  let (sender, reader) = stream.split();

  Ok(Connection { sender, reader })
}

pub struct Connection {
  pub sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
  pub reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}
