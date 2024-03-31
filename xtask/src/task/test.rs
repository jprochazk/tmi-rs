use crate::util::{cargo, CommandExt};
use crate::Result;
use argp::FromArgs;
use std::process::Command;

#[derive(FromArgs)]
#[argp(subcommand, name = "test", description = "Run tests")]
pub struct Test {}

impl Test {
  pub fn run(self) -> Result {
    tests().run()?;

    Ok(())
  }
}

fn tests() -> Command {
  cargo("insta").with_args(["test", "--all-features", "--lib", "--review"])
}
