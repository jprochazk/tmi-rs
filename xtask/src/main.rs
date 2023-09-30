mod task;
mod util;

use self::task::Task;
use argp::FromArgs;
use std::io::{stderr, Write};
use std::process::ExitCode;

pub type Result<T = (), E = anyhow::Error> = anyhow::Result<T, E>;

#[derive(FromArgs)]
#[argp(description = "Common tasks")]
pub struct Cli {
  #[argp(subcommand)]
  pub task: Task,
}

fn try_main() -> Result {
  let args: Cli = argp::parse_args_or_exit(argp::DEFAULT);
  args.task.run()
}

fn main() -> ExitCode {
  match try_main() {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
      let _ = write!(stderr(), "{e}");
      ExitCode::FAILURE
    }
  }
}
