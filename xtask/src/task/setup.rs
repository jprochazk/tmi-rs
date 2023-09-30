use crate::util::{cargo, has_cargo_subcmd, rustup, CommandExt};
use crate::Result;
use argp::FromArgs;

const COMPONENTS: &[&str] = &["rustfmt", "clippy"];
const TOOLS: &[&str] = &["cargo-insta"];

#[derive(FromArgs)]
#[argp(subcommand, name = "setup")]
/// Install tooling
pub struct Setup {
  #[argp(switch, description = "Install using `cargo-binstall` instead")]
  binary: bool,
}

impl Setup {
  pub fn run(self) -> Result {
    if self.binary {
      if !has_cargo_subcmd("binstall")? {
        cargo("install").with_arg("cargo-binstall").run()?;
      }

      cargo("binstall")
        .with_args(["--no-confirm", "--locked"])
        .with_args(TOOLS)
        .run()?;
    } else {
      cargo("install")
        .with_args(["--locked"])
        .with_args(TOOLS)
        .run()?;
    }

    rustup("component")
      .with_arg("add")
      .with_args(COMPONENTS)
      .run()?;

    Ok(())
  }
}
