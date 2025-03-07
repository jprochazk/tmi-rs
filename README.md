# `tmi-rs` &emsp; [![Documentation]][docs.rs] [![Latest Version]][crates.io]

[docs.rs]: https://docs.rs/tmi/latest/tmi/
[crates.io]: https://crates.io/crates/tmi
[Documentation]: https://img.shields.io/docsrs/tmi
[Latest Version]: https://img.shields.io/crates/v/tmi.svg

[Blazingly fast](#performance) 🚀 Rust 🦀 library for interacting with [twitch.tv](https://twitch.tv)'s chat interface.

## Quick Start

```text,ignore
$ cargo add tmi anyhow tokio -F tokio/full
```

```rust,no_run
const CHANNELS: &[&str] = &["#forsen"];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let mut client = tmi::Client::anonymous().await?;
  client.join_all(CHANNELS).await?;

  loop {
    let msg = client.recv().await?;
    match msg.as_typed()? {
      tmi::Message::Privmsg(msg) => {
        println!("{}: {}", msg.sender().name(), msg.text());
      }
      tmi::Message::Reconnect => {
        client.reconnect().await?;
        client.join_all(CHANNELS).await?;
      }
      tmi::Message::Ping(ping) => {
        client.pong(&ping).await?;
      }
      _ => {}
    }
  }
}
```

## Performance

Calling the library blazingly fast is done in jest, but it is true that `tmi-rs` is very fast. `tmi-rs` is part of the [twitch-irc-benchmarks](https://github.com/jprochazk/twitch-irc-benchmarks), where it is currently the fastest implementation by a significant margin (nearly 6x faster than the second best Rust implementation). This is because underlying IRC message parser is handwritten and accelerated using SIMD on x86 and ARM. For every other architecture, there is a scalar fallback.

## Acknowledgements

Initially based on [dank-twitch-irc](https://github.com/robotty/dank-twitch-irc), and [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs). Lots of test messages were taken directly from [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs).
