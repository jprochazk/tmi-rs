use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;
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
async fn main() -> ExitCode {
  if let Err(e) = try_main().await {
    eprintln!("{e}");
    return ExitCode::FAILURE;
  }

  ExitCode::SUCCESS
}

async fn try_main() -> Result<()> {
  let channel = std::env::args().nth(1).ok_or("missing argument #channel")?;

  let config_path = home::home_dir()
    .ok_or("failed to get home dir")?
    .join(".nanochat");

  let channel = channel.trim();
  let channel = if channel.starts_with('#') {
    channel.to_string()
  } else {
    format!("#{channel}")
  };

  println!("> Joining {channel}");

  let mut ws = connect(&channel).await?;
  let mut state = State::init(channel, &config_path)?;

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

  state.save(&config_path)?;

  Ok(())
}

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
  ::core::result::Result<T, E>;

type WebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct State {
  channel: String,
  message_counts: HashMap<String, usize>,
  largest_message_count: usize,
  uwu: HashSet<String>,
  ban: HashSet<String>,
}

impl State {
  fn init(channel: String, config_path: &Path) -> Result<State> {
    let content = if config_path.try_exists()? {
      fs::read_to_string(config_path)?
    } else {
      String::new()
    };

    let mut uwu = HashSet::new();
    let mut ban = HashSet::new();

    for line in content.trim().lines() {
      let Some((key, value)) = line.split_once(':') else {
        println!("! invalid config line: {line}");
        continue;
      };
      let (key, value) = (key.trim(), value.trim());
      match key {
        "uwu" => uwu.insert(value.to_string()),
        "ban" => ban.insert(value.to_string()),
        _ => {
          println!("! unrecognized config key: {key} (in line `{line}`)");
          continue;
        }
      };
    }

    Ok(State {
      channel,
      message_counts: HashMap::new(),
      largest_message_count: 0,
      uwu,
      ban,
    })
  }

  fn save(&self, config_path: &Path) -> Result<()> {
    use std::fmt::Write;

    let mut contents = String::new();
    for name in self.uwu.iter() {
      writeln!(&mut contents, "uwu:{name}")?;
    }
    for word in self.ban.iter() {
      writeln!(&mut contents, "ban:{word}")?;
    }
    fs::write(config_path, contents)?;

    Ok(())
  }

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
}

async fn connect(channel: &str) -> Result<WebSocket> {
  let (mut ws, _) = tokio_tungstenite::connect_async("ws://irc-ws.chat.twitch.tv:80").await?;

  println!("> Authenticating");
  ws.send(Message::Text(
    "CAP REQ :twitch.tv/commands twitch.tv/tags".into(),
  ))
  .await?;
  ws.send(Message::Text("PASS just_a_lil_guy".into())).await?;
  ws.send(Message::Text("NICK justinfan83124".into())).await?;
  ws.send(Message::Text(format!("JOIN {channel}"))).await?;

  println!("> Connected");

  Ok(ws)
}

async fn reconnect(ws: &mut WebSocket, channel: &str) -> Result<()> {
  let mut tries = 10;
  let mut delay = Duration::from_secs(3);

  println!("> Reconnecting");
  tokio::time::sleep(delay).await;

  loop {
    match connect(channel).await {
      Ok(new_socket) => {
        *ws = new_socket;
        break Ok(());
      }
      Err(e) if tries > 0 => {
        tries -= 1;
        delay *= 3;
        println!("> Connection failed: {e}");
        println!("> Retrying...");
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
        Command::Reconnect => reconnect(ws, &state.channel).await?,
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
  args: &str,
) -> Result<()> {
  let is_privileged = ["moscowwbish", &state.channel].contains(&login);

  let args = args.trim();

  match cmd {
    "uwu" => {
      if state.uwu.contains(login) {
        println!("uwu 👉 {login}");
        state.uwu.remove(login)
      } else {
        println!("uwu 🤚 {login}");
        state.uwu.insert(login.to_string())
      };
    }
    "bad" if is_privileged => {
      let word = args.split_whitespace().next().unwrap_or(args);
      println!("> Added `{word}` to bad words");
      state.ban.insert(word.to_string());
    }
    "unbad" if is_privileged => {
      let word = args.split_whitespace().next().unwrap_or(args);
      println!("> Removed `{word}` from bad words");
      state.ban.remove(word);
    }
    _ => {}
  };

  Ok(())
}

fn normal_message(state: &mut State, name: &str, text: &str) -> Result<()> {
  let name = to_lowercase(name);

  for word in text.split_whitespace() {
    if state.ban.contains(word) {
      return Ok(());
    }
  }

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
