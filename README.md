[Blazingly fast](#performance) 🚀 Rust 🦀 library for interacting with [twitch.tv](https://twitch.tv)'s chat interface.

```
$ cargo add tmi
```

```rust,no_run
let mut client = tmi::Client::connect().await?;
client.join("#moscowwbish").await?;

loop {
  let msg = client.message().await?;
  match tmi::Message::try_from(msg.as_ref())? {
    tmi::Message::Privmsg(msg) => println!("{}: {}", msg.sender().name(), msg.text()),
    tmi::Message::Reconnect => client.reconnect().await?,
    tmi::Message::Ping(ping) => client.pong(&ping).await?,
    _ => {}
  };
}
```

## Performance

Calling the library blazingly fast is done in jest, but it is true that `twitch-rs` is very fast. `twitch-rs` is part of the [twitch-irc-benchmarks](https://github.com/jprochazk/twitch-irc-benchmarks), where it is currently the fastest implementation by a significant margin (roughly 2.5x faster than the second best implementation). The reason for this is that the IRC message parser is handwritten using SIMD instructions for x86 and ARM. For every other architecture, there is a scalar fallback. The SIMD implementation is quite portable, as on x86 it relies only on SSE2, which is available in every x86 CPU created in the last two decades, and on ARM it relies on Neon, which is also well supported.

## Acknowledgements

Initially based on [dank-twitch-irc](https://github.com/robotty/dank-twitch-irc), and [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs). Lots of test messages were taken directly from [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs).
