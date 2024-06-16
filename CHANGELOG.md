## Unreleased

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
