[Blazingly fast](#performance) 🚀 Twitch IRC parsing library written in Rust 🦀

This is a Twitch-specific IRC parser, it is not guaranteed to work for any other kind of IRC message.
It is also fairly low level, and doesn't provide a convenient high-level interface to build bots with.

```
$ cargo add --git https://github.com/jprochazk/twitch-rs.git
```

```rust
use twitch::{Command, Tag};

for message in data.lines().flat_map(twitch::parse) {
  if message.command() == Command::Privmsg {
    let name = message.tag(Tag::DisplayName).unwrap();
    let text = message.text().unwrap();
    println!("{name}: {text}");
  }
}
```

If your cpu supports SSE2 (x86) or NEON (arm), you can enable the `simd` feature for a ~50% increase in performance:
```
$ cargo add --git https://github.com/jprochazk/twitch-rs.git -F simd
```

## Performance

Calling the library blazingly fast is done in jest, but it is true that `twitch-rs` is very fast. `twitch-rs` is part of the [twitch-irc-benchmarks](https://github.com/jprochazk/twitch-irc-benchmarks), where it is currently the fastest implementation by a significant margin, nearly twice as fast as the second best implementation.

The SIMD implementation is quite portable, as on x86 it relies only on SSE2, which is available in every x86 CPU created in the last two decades, and on ARM it relies on Neon, which is also well supported.

## Acknowledgements

Initially based on [dank-twitch-irc](https://github.com/robotty/dank-twitch-irc), and [twitch-irc-rs](https://github.com/robotty/twitch-irc-rs).
