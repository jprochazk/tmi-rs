use tokio::select;
use tokio::signal::ctrl_c;
use twitch::client::Client;
use twitch::{Command, IrcMessage};

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
  ::core::result::Result<T, E>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let mut client = Client::connect(Default::default()).await?;

  client.join_all(["#riotgames"]).await?;

  loop {
    select! {
      _ = ctrl_c() => {
        break;
      }
      message = client.message() => {
        handle_message(&mut client, message?).await?;
      }
    }
  }

  Ok(())
}

async fn handle_message(client: &mut Client, message: IrcMessage) -> Result<()> {
  let a = message.as_ref();
  let b = twitch_irc::message::IRCMessage::parse(message.raw()).unwrap();

  let tags = a.tags().collect::<Vec<_>>();

  assert_eq!(a.command().as_str(), b.command);
  assert_eq!(tags.is_empty(), b.tags.0.is_empty());

  if !tags.is_empty() {
    assert_eq!(tags.len(), b.tags.0.len());
    print!("tags{{");
    for (tag, value) in tags {
      match b.tags.0.get(tag.as_str()).unwrap() {
        Some(other) => assert_eq!(&twitch::unescape(value), other),
        None => assert!(value.is_empty()),
      }

      print!("{}={};", tag.as_str(), twitch::unescape(value));
    }
    print!("}} ");
  }

  if let Some(prefix) = a.prefix() {
    print!("{prefix} ");
  }
  print!("{} ", a.command());
  if let Some(channel) = a.channel() {
    print!("{channel} ");
  }
  if let Some(params) = a.params() {
    print!("{params} ");
  }
  println!();

  if a.command() == Command::Ping {
    client.send("PONG\r\n").await?;
  }

  Ok(())
}
