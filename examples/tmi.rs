use twitch::tmi::{self, Message};

fn init_logger() -> std::result::Result<(), alto_logger::Error> {
  if std::env::var("RUST_LOG").is_err() {
    std::env::set_var("RUST_LOG", "DEBUG");
  }
  alto_logger::init_term_logger()
}

#[tokio::main]
async fn main() {
  init_logger().unwrap();

  let mut conn = tmi::connect(tmi::Config::default()).await.unwrap();
  conn.sender.join("moscowwbish").await.unwrap();

  loop {
    tokio::select! {
      _ = tokio::signal::ctrl_c() => {
        log::info!("CTRL-C");
        break;
      },
      result = conn.reader.next() => match result {
        Ok(message) => match message {
          Message::Ping(ping) => conn.sender.pong(ping.arg()).await.unwrap(),
          /* Message::Privmsg(message) => {
            log::info!("#{} {} ({}): {}", message.channel(), message.user.name, message.user.id(), message.text());
            if message.text().starts_with("!stop") {
              break;
            }
          }, */
          _ => {
            log::info!("{:?}", message)
          }
        },
        Err(err) => {
          panic!("{}", err);
        }
      }
    }
  }
}
