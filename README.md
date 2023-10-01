[Blazingly fast](#performance) 🚀 Rust 🦀 library for interacting with [twitch.tv](https://twitch.tv)'s chat interface.

```text,ignore
$ cargo add tmi
```

```rust,ignore
let mut client = tmi::Client::connect().await?;
client.join("#moscowwbish").await?;

loop {
  let msg = client.recv().await?;
  match msg.as_typed()? {
    tmi::Message::Privmsg(msg) => println!("{}: {}", msg.sender().name(), msg.text()),
    tmi::Message::Reconnect => client.reconnect().await?,
    tmi::Message::Ping(ping) => client.pong(&ping).await?,
    _ => {}
  };
}
```

## Performance

Calling the library blazingly fast is done in jest, but it is true that `tmi-rs` is very fast. `tmi-rs` is part of the [twitch-irc-benchmarks](https://github.com/jprochazk/twitch-irc-benchmarks), where it is currently the fastest implementation by a significant margin (roughly 2.5x faster than the second best implementation). The reason for this is that the IRC message parser is handwritten using SIMD instructions for x86 and ARM. For every other architecture, there is a scalar fallback. The SIMD implementation is quite portable, as on x86 it relies only on SSE2, which is available in every x86 CPU created in the last two decades, and on ARM it relies on Neon, which is also well supported.

## Acknowledgements

Initially based on [dank-twitch-irc](https://github.com/robotty/dank-twitch-irc), and [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs). Lots of test messages were taken directly from [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs).
