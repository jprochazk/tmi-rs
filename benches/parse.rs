use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn read_input() -> Vec<String> {
  include_str!("data.txt")
    .lines()
    .take(1000)
    .map(String::from)
    .collect::<Vec<_>>()
}

fn twitch(c: &mut Criterion) {
  let input = read_input();
  c.bench_with_input(
    BenchmarkId::new("twitch", "data.txt (whitelist)"),
    &input,
    |b, lines| {
      b.iter_with_setup(
        || lines.clone(),
        |lines| {
          for line in lines {
            black_box(
              twitch::Message::parse_with_whitelist(line, twitch::whitelist!(TmiSentTs, UserId))
                .expect("failed to parse"),
            );
          }
        },
      );
    },
  );
  c.bench_with_input(
    BenchmarkId::new("twitch", "data.txt (no whitelist)"),
    &input,
    |b, lines| {
      b.iter_with_setup(
        || lines.clone(),
        |lines| {
          for line in lines {
            black_box(twitch::Message::parse(line).expect("failed to parse"));
          }
        },
      );
    },
  );
}

criterion_group!(benches, twitch);
criterion_main!(benches);
