use anyhow::Result;
use clap::Parser;
use tokio::select;
use tokio::signal::ctrl_c;

#[derive(Parser)]
#[command(author, version)]
struct Args {
  /// Login username
  #[arg(long)]
  nick: Option<String>,

  /// Login oauth2 token
  #[arg(long)]
  token: Option<String>,

  /// Channels to join
  #[arg(long)]
  channel: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();

  let credentials = match args.nick.zip(args.token) {
    Some((nick, token)) => tmi::client::Credentials::new(nick, token),
    None => tmi::client::Credentials::anon(),
  };

  let channels = args
    .channel
    .into_iter()
    .map(tmi::Channel::parse)
    .collect::<Result<Vec<_>, _>>()?;

  println!("Connecting as {}", credentials.nick);
  let mut client = tmi::Client::builder()
    .credentials(credentials)
    .connect()
    .await?;

  client.join_all(&channels).await?;
  println!("Joined the following channels: {}", channels.join(", "));

  select! {
    _ = ctrl_c() => {
      Ok(())
    }
    res = tokio::spawn(run(client, channels)) => {
      res?
    }
  }
}

async fn run(mut client: tmi::Client, channels: Vec<tmi::Channel>) -> Result<()> {
  loop {
    let msg = client.recv().await?;
    match msg.as_typed()? {
      tmi::Message::Privmsg(msg) => on_msg(&mut client, msg).await?,
      tmi::Message::Reconnect => {
        client.reconnect().await?;
        client.join_all(&channels).await?;
      }
      tmi::Message::Ping(ping) => client.pong(&ping).await?,
      _ => {}
    };
  }
}

async fn on_msg(client: &mut tmi::Client, msg: tmi::Privmsg<'_>) -> Result<()> {
  println!("{}: {}", msg.sender().name(), msg.text());

  if client.credentials().is_anon() {
    return Ok(());
  }

  if !msg.text().starts_with("!yo") {
    return Ok(());
  }

  client
    .privmsg(msg.channel(), "yo")
    .reply_to(msg.message_id())
    .send()
    .await?;

  println!("< {} yo", msg.channel());

  Ok(())
}
