## 0.9.0

* Support `channel` getter for `Names` and `EndOfNames` [17df3a8](https://github.com/jprochazk/tmi-rs/commit/17df3a8)
* Return all params from `text` if trailer is missing [5f240c8](https://github.com/jprochazk/tmi-rs/commit/5f240c8)

Full commit range: [0.8.0..bf3e823](https://github.com/jprochazk/tmi-rs/compare/0.8.0...bf3e823)

## 0.8.0

* Return `User` and `SubGiftPromo` by ref [b0f0157](https://github.com/jprochazk/tmi-rs/commit/b0f0157)

Full commit range: [0.7.3..3d497e1](https://github.com/jprochazk/tmi-rs/compare/0.7.3...3d497e1)

## 0.7.3

* Fix typed message parsing for announcement `USERNOTICE`s without `msg-param-color` tag [38a2173](https://github.com/jprochazk/tmi-rs/commit/38a2173) by [ByteZ1337](https://github.com/ByteZ1337)
  * Now defaults to `PRIMARY` if not present

## 0.7.2

* Fix equals in tag value regression again [1a0ba79](https://github.com/jprochazk/tmi-rs/commit/1a0ba79)

Full commit range: [0.7.1..3d497e1](https://github.com/jprochazk/tmi-rs/compare/0.7.1...3d497e1)

## 0.7.1

This release include another full rewrite of the tag parser, using a new approach that resulted
in an average 50% performance improvement over version `0.7.0`.

```
# Baseline: f5c6c32da475a7436c0aa58e4f24874364955dcf

# ARM NEON
twitch/1000 
  before: 245.584 µs
  after:  121.391 µs
  change: -49.4%

# x86 AVX512
twitch/1000
  before: 188.064 µs
  after:   94.260 µs
  change: -50.1%
```

x86 now has implementations using SSE2, AVX2, and AVX512, choosing the best available at compile time.
For that reason, the crate should ideally be compiled with `RUSTFLAGS="-C target-cpu=native"`.

Full commit range: [0.7.0..3b19a23](https://github.com/jprochazk/tmi-rs/compare/0.7.0...3b19a23)

## 0.7.0

This release adds support for a few new tags, and changes the names of some typed message fields
to better match the tag names used by Twitch.

### New tags

- `pinned-chat-paid` on `Privmsg`
- `msg-id` on `Privmsg`

### Breaking changes

- `message_id` on `Privmsg` is now `id`
- `message_id` on `ClearMsg` is now `target_message_id`
- `tags` on `IrcMessage`/`IrcMessageRef` now returns string slices for keys
  - You can use `tmi::Tag::parse` to continue using the enum in your match statements

### Performance

This release includes a full rewrite of the tag parser, which resulted in a ~15% performance improvement.

Full commit range: [0.6.1..f5e539f](https://github.com/jprochazk/tmi-rs/compare/0.6.1...f5e539f)

## 0.6.1

This is a bugfix release with no new features or breaking changes.

### Fixes

- Under certain conditions, the SIMD version of the prefix parser would cause a panic.
  It has been disabled until the issue can be resolved.

Full commit range: [0.6.0..36f8210](https://github.com/jprochazk/tmi-rs/compare/0.6.0...36f8210)
