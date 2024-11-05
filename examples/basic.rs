//! Basic usage example.
//!
//! ```text,ignore
//! $ cargo run --example basic -- \
//!   --token oauth:yfvzjqb705z12hrhy1zkwa9xt7v662 \
//!   --channel #forsen
//! ```

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(author, version)]
struct Args {
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
  tmi::run_in_place(args.channel, on_msg).await?;
  Ok(())
}

async fn on_msg(ctx: tmi::Context, msg: tmi::Privmsg<'_>) -> Result<(), tmi::BotError> {
  println!("{}: {}", msg.sender().name(), msg.text());

  if ctx.is_anon() {
    return Ok(());
  }

  if !msg.text().starts_with("!yo") {
    return Ok(());
  }

  ctx.privmsg(msg.channel(), "yo").reply_to(msg.id()).send();

  println!("< {} yo", msg.channel());

  Ok(())
}
