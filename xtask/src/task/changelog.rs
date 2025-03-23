use crate::util::{git, CommandExt};
use crate::Result;
use argp::FromArgs;

#[derive(FromArgs)]
#[argp(subcommand, name = "changelog")]
/// Generate a changelog
pub struct Changelog {
  #[argp(option, description = "The tag to start from (inclusive)")]
  since: String,

  #[argp(option, description = "The tag to end at (inclusive)")]
  until: Option<String>,
}

impl Changelog {
  pub fn run(self) -> Result {
    // git log 0.6.1..HEAD --pretty=format:"%h"
    let git_log = git("log")
      .with_args([
        &format!(
          "{}..{}",
          self.since,
          self.until.as_deref().unwrap_or("HEAD")
        ),
        "--pretty=format:%h",
      ])
      .run_with_output()?;
    let commits: Vec<&str> = git_log.trim().split('\n').collect();

    let mut lines = vec![];
    for commit in commits {
      let message = git("show")
        .with_args(["--quiet", "--pretty=format:%B", commit])
        .run_with_output()?;
      let (first_line, remainder) = message.split_once('\n').unwrap_or((&message, ""));
      let url = format!("https://github.com/jprochazk/tmi-rs/commit/{}", commit);
      lines.push(format!("{} [{}]({})", first_line, commit, url,));
      lines.push(remainder.to_string());
    }

    let latest_commit = git("log")
      .with_args(["-1", "--pretty=format:%h"])
      .run_with_output()?;
    let gh_commit_range = format!(
      "[{0}..{1}](https://github.com/jprochazk/tmi-rs/compare/{0}...{1})",
      self.since, latest_commit
    );

    let existing_changelog = std::fs::read_to_string("CHANGELOG.md").unwrap();

    std::fs::write(
      "CHANGELOG.md",
      format!(
        "{}\n\n{}\n\n{existing_changelog}",
        gh_commit_range,
        lines.join("\n")
      ),
    )?;

    Ok(())
  }
}
