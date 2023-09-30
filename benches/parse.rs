use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mimalloc::MiMalloc;
use twitch::IrcMessageRef;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn read_input() -> Vec<String> {
  include_str!("data.txt")
    .lines()
    .map(String::from)
    .collect::<Vec<_>>()
}

fn run_bench<'a, I, P>(c: &mut Criterion, name: &str, input: I, parser: P)
where
  I: IntoIterator<Item = &'a str> + Clone,
  P: Fn(&'a str) -> Option<IrcMessageRef<'a>>,
{
  c.bench_with_input(BenchmarkId::new("twitch", name), &input, |b, input| {
    b.iter_with_setup(
      || input.clone(),
      |input| {
        for line in input {
          let msg = parser(line).expect("failed to parse");
          black_box(msg);
        }
      },
    );
  });
}

macro_rules! run {
  ($c:ident, $input:ident, $name:literal, $count:expr, $parser:expr) => {{
    let input = $input.iter().map(String::as_str).take($count);
    run_bench($c, $name, input, $parser);
  }};
}

fn twitch(c: &mut Criterion) {
  let input = read_input();

  run!(c, input, "1000", 1000, IrcMessageRef::parse);
  run!(c, input, "100000", 100000, IrcMessageRef::parse);
  run!(c, input, "all", input.len(), IrcMessageRef::parse);
}

criterion_group!(benches, twitch);
criterion_main!(benches);
