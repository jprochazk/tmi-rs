use std::collections::HashMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use twitch::Command;

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
  ::core::result::Result<T, E>;

type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
struct State {
  message_counts: HashMap<String, u64>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  println!("Connecting");
  let mut ws = connect().await?;
  let mut state = State::default();

  loop {
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {
        break;
      }
      Some(message) = ws.next() => {
        handle_message(&mut ws, &mut state, message?).await?;
      }
    }
  }

  Ok(())
}

async fn connect() -> Result<WebSocket> {
  let (mut ws, _) = tokio_tungstenite::connect_async("ws://irc-ws.chat.twitch.tv:80").await?;

  println!("Authenticating");
  ws.send(Message::Text(
    "CAP REQ :twitch.tv/commands twitch.tv/tags".into(),
  ))
  .await?;
  ws.send(Message::Text("PASS just_a_lil_guy".into())).await?;
  ws.send(Message::Text("NICK justinfan83124".into())).await?;
  ws.send(Message::Text("JOIN #kirinokirino".into())).await?;

  println!("Connected");

  Ok(ws)
}

async fn reconnect(ws: &mut WebSocket) -> Result<()> {
  let mut tries = 10;
  let mut delay = Duration::from_secs(3);

  println!("Reconnecting");
  tokio::time::sleep(delay).await;

  loop {
    match connect().await {
      Ok(new_socket) => {
        *ws = new_socket;
        break Ok(());
      }
      Err(e) if tries > 0 => {
        tries -= 1;
        delay *= 3;
        println!("Connection failed: {e}");
        println!("Retrying...");
        tokio::time::sleep(delay).await;
        continue;
      }
      Err(e) => {
        break Err(format!("failed to reconnect: {e}").into());
      }
    }
  }
}

async fn handle_message(ws: &mut WebSocket, state: &mut State, msg: Message) -> Result<()> {
  if let Message::Text(msg) = msg {
    for msg in msg.lines() {
      let msg = twitch::Message::parse(msg)?;

      match msg.command() {
        Command::Privmsg => {
          use twitch::Tag::*;

          let name = msg.tag(DisplayName).unwrap_or("???").to_lowercase();
          let text = msg.text().unwrap_or("???");
          let count = state.message_counts.entry(name.to_string()).or_insert(0);
          println!("{count} {name}:\t{text}");

          *count += 1;
        }
        Command::Ping => ws.send(Message::Text("PONG".into())).await?,
        Command::Reconnect => reconnect(ws).await?,
        _ => {}
      }
    }
  }

  Ok(())
}
