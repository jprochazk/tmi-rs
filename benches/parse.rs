use std::str::FromStr;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn parse(c: &mut Criterion) {
  let data = std::fs::read_to_string("benches/data.txt")
    .unwrap()
    .lines()
    .take(1000)
    .map(String::from)
    .collect::<Vec<_>>();

  c.bench_with_input(BenchmarkId::new("twitch", "data.txt"), &data, |b, lines| {
    b.iter(|| {
      for line in lines.clone() {
        black_box(twitch::Message::parse(line).expect("failed to parse"));
      }
    })
  });

  c.bench_with_input(
    BenchmarkId::new("twitch_irc", "data.txt"),
    &data,
    |b, lines| {
      b.iter(|| {
        for line in lines.clone() {
          black_box(twitch_irc::message::IRCMessage::parse(&line).expect("failed to parse"));
        }
      })
    },
  );

  c.bench_with_input(
    BenchmarkId::new("irc_rust", "data.txt"),
    &data,
    |b, lines| {
      b.iter(|| {
        for line in lines.clone() {
          black_box(
            irc_rust::Message::from_str(&line)
              .expect("failed to parse")
              .parse()
              .expect("failed to parse"),
          );
        }
      })
    },
  );
}

criterion_group!(bench, parse);
criterion_main!(bench);
