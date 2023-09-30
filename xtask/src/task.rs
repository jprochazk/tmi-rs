use crate::Result;
use argp::FromArgs;

mod setup;
mod test;

#[derive(FromArgs)]
#[argp(subcommand)]
pub enum Task {
  Setup(setup::Setup),
  Test(test::Test),
}

impl Task {
  pub fn run(self) -> Result {
    use Task as T;
    match self {
      T::Setup(task) => task.run(),
      T::Test(task) => task.run(),
    }
  }
}
