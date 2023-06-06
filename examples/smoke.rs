use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
  ::core::result::Result<T, E>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let mut ws = tokio_tungstenite::connect_async("ws://irc-ws.chat.twitch.tv:80")
    .await?
    .0;

  ws.send(Message::Text(
    "CAP REQ :twitch.tv/commands twitch.tv/tags".into(),
  ))
  .await?;
  ws.send(Message::Text("PASS just_a_lil_guy".into())).await?;
  ws.send(Message::Text("NICK justinfan83124".into())).await?;
  ws.send(Message::Text("JOIN #anny".into())).await?;

  loop {
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {
        break;
      }
      Some(message) = ws.next() => {
        let message = message?;
        handle_message(message)?;
      }
    }
  }

  Ok(())
}

fn handle_message(message: Message) -> Result<()> {
  if let Message::Text(message) = message {
    for line in message.lines() {
      println!("\n{}", line);

      let a = twitch::Message::parse(line).unwrap();
      let b = twitch_irc::message::IRCMessage::parse(line).unwrap();

      assert_eq!(a.command().as_str(), b.command);
      assert_eq!(
        a.tags().is_some() && !a.tags().unwrap().is_empty(),
        !b.tags.0.is_empty()
      );

      if let Some(tags) = a.tags() {
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
      print!(
        "{} {} {}",
        a.command(),
        a.channel().unwrap_or("<no channel>"),
        a.params().unwrap_or("")
      );

      println!();
    }
  }

  Ok(())
}
