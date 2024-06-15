use crate::Result;
use argp::FromArgs;

mod changelog;
mod setup;
mod test;

#[derive(FromArgs)]
#[argp(subcommand)]
pub enum Task {
  Setup(setup::Setup),
  Test(test::Test),
  Changelog(changelog::Changelog),
}

impl Task {
  pub fn run(self) -> Result {
    use Task as T;
    match self {
      T::Setup(task) => task.run(),
      T::Test(task) => task.run(),
      T::Changelog(task) => task.run(),
    }
  }
}
