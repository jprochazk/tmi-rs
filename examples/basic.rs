//! Basic usage example.
//!
//! ```text,ignore
//! $ cargo run --example basic -- \
//!   --login your_user_name \
//!   --token oauth:yfvzjqb705z12hrhy1zkwa9xt7v662 \
//!   --channel #pajlada
//! ```

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(author, version)]
struct Args {
  /// Login username
  #[arg(long)]
  login: Option<String>,

  /// Login oauth2 token
  #[arg(long)]
  token: Option<String>,

  /// Channels to join
  #[arg(long = "channel")]
  channels: Vec<String>,
}

fn init_tracing() {
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "trace");
  }
  tracing_subscriber::fmt::init();
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  init_tracing();

  let Args {
    login,
    token,
    channels,
  } = Args::parse();

  tmi::Bot::new()
    .auth(login.zip(token))
    .channels(channels)
    .run_in_place(handler)
    .await?;

  Ok(())
}

async fn handler(_: tmi::Context, m: tmi::Privmsg<'_>) -> Result<(), tmi::BotError> {
  println!("{}: {}", m.sender().name(), m.text());

  Ok(())
}
