# Benchmarks

The general outline of the benchmark:
- The first 1000 lines of `data.txt` are read and prepared during setup
- For each benchmark iteration, parse each line in a loop and discard the result

`data.txt` consists of roughly 200 thousand messages. The benchmarks only read the first 1000 lines from this file.

### Rust

Rust benchmarks use [criterion](https://github.com/bheisler/criterion.rs).

```
$ cargo bench
```

### C# .NET

C# benchmarks use [BenchmarkDotNet](https://github.com/dotnet/BenchmarkDotNet).

```
$ cd dotnet && dotnet run -c Release
```

### Go

Go benchmarks use the built-in benchmarking tool.

```
$ cd go && go test -bench=.
```

## Results

Benchmarks were run in WSL2 Ubuntu 22.04 on an AMD Ryzen 7950X

| library                                                                                            | language                                 | time to parse 1000 lines |
| -------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------ |
| [twitch](https://github.com/jprochazk/twitch-rs/tree/68710b3950d1369b4a59990860d48f05f37d9182)     | Rust 1.72-nightly (871b59520 2023-05-31) | 569.69 µs                |
| [twitch-irc](https://github.com/robotty/twitch-irc-rs/tree/v5.0.0)                                 | Rust 1.72-nightly (871b59520 2023-05-31) | 2.2108 ms                |
| [irc_rust](https://github.com/MoBlaa/irc_rust/tree/4ae66fb3176b1d46cec6764f1a76aa6e9673d08b)       | Rust 1.72-nightly (871b59520 2023-05-31) | 969.26 µs                |
| [justgrep](https://github.com/Mm2PL/justgrep/tree/v0.0.6)                                          | Go 1.20                                  | 1.391626 ms              |
| [minitwitch](https://github.com/Foretack/MiniTwitch/tree/ce17607da83d70e05e2d2cec873d4182abfc03eb) | .NET 6.0                                 | 1.217 ms                 |
