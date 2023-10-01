use tokio::select;
use tokio::signal::ctrl_c;

type Result<T, E = Box<dyn std::error::Error + Send + Sync + 'static>> =
  ::core::result::Result<T, E>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let mut client = tmi::Client::connect().await?;
  client.join("#forsen").await?;

  loop {
    select! {
      _ = ctrl_c() => {
        break;
      }
      msg = client.recv() => {
        match msg?.as_ref().as_typed()? {
          tmi::Message::Privmsg(msg) => println!("{}: {}", msg.sender().name(), msg.text()),
          tmi::Message::Reconnect => client.reconnect().await?,
          tmi::Message::Ping(ping) => client.pong(&ping).await?,
          _ => {}
        };
      }
    }
  }

  Ok(())
}
