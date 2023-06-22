use std::collections::{HashMap, HashSet};
use std::time::Duration;

use beef::lean::Cow;
use futures_util::{SinkExt, StreamExt};
use mimalloc::MiMalloc;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use twitch::Command;
use uwuifier::uwuify_str_sse;

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

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
  ::core::result::Result<T, E>;

type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
struct State {
  message_counts: HashMap<String, usize>,
  largest_message_count: usize,
  uwu: HashSet<String>,
}

impl State {
  fn count_width(&self) -> usize {
    num_digits(self.largest_message_count)
  }

  fn uwuify<'a>(&self, name: &str, text: &'a str) -> Cow<'a, str> {
    if self.uwu.contains(name) {
      Cow::owned(uwuify_str_sse(text))
    } else {
      Cow::borrowed(text)
    }
  }

  fn is_uwu(&self, name: &str) -> bool {
    self.uwu.contains(name)
  }

  fn uwu(&mut self, name: String) {
    self.uwu.insert(name);
  }

  fn unuwu(&mut self, name: &str) {
    self.uwu.remove(name);
  }
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

          let login = msg.prefix().and_then(|v| v.nick).unwrap_or("???");
          let name = msg.tag(DisplayName).unwrap_or("???");
          let text = msg.text().unwrap_or("???").trim();

          match text.strip_prefix('!').map(split_args) {
            Some((cmd, args)) => invoke_command(ws, state, login, cmd, args).await?,
            None => normal_message(state, name, text)?,
          }
        }
        Command::Ping => ws.send(Message::Text("PONG".into())).await?,
        Command::Reconnect => reconnect(ws).await?,
        _ => {}
      }
    }
  }

  Ok(())
}

async fn invoke_command(
  _ws: &mut WebSocket,
  state: &mut State,
  login: &str,
  cmd: &str,
  _args: &str,
) -> Result<()> {
  match cmd {
    "uwu" if state.is_uwu(login) => state.unuwu(login),
    "uwu" => state.uwu(login.to_string()),
    _ => {}
  }

  Ok(())
}

fn normal_message(state: &mut State, name: &str, text: &str) -> Result<()> {
  let name = to_lowercase(name);
  let text = state.uwuify(&name, text);
  let width = state.count_width();
  let count = state.message_counts.entry(name.to_string()).or_insert(0);
  println!("{count:width$} {name}:\t{text}");

  *count += 1;
  state.largest_message_count = std::cmp::max(*count, state.largest_message_count);

  Ok(())
}

fn split_args(s: &str) -> (&str, &str) {
  s.split_once(' ').unwrap_or((s, ""))
}

fn to_lowercase(s: &str) -> Cow<str> {
  if s.chars().all(|c| c.is_lowercase()) {
    Cow::borrowed(s)
  } else {
    Cow::owned(s.to_lowercase())
  }
}

fn num_digits(v: usize) -> usize {
  std::iter::successors(Some(v), |&n| (n >= 10).then_some(n / 10)).count()
}
