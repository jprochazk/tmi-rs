use crate::util::{cargo, CommandExt};
use crate::Result;
use argp::FromArgs;
use std::process::Command;

#[derive(FromArgs)]
#[argp(subcommand, name = "test", description = "Run tests")]
pub struct Test {
  /// Additional arguments for the test command
  #[argp(positional)]
  rest: Vec<String>,
}

impl Test {
  pub fn run(self) -> Result {
    tests(&self.rest).run()?;

    Ok(())
  }
}

fn tests(args: &[String]) -> Command {
  cargo("insta")
    .with_args(["test", "--all-features", "--lib", "--review"])
    .with_args(args)
}
