# Benchmarks

This repository only holds benchmarks used to guide performance work on `twitch-rs`. To see `twitch-rs` compared against other libraries (including other languages), go to [twitch-irc-benchmarks](https://github.com/jprochazk/twitch-irc-benchmarks).

The benchmarks use [criterion](https://github.com/bheisler/criterion.rs).

Running the benchmarks:

```
$ cargo bench
```

Running the benchmarks with simd enabled:

```
$ cargo bench -F simd
```
