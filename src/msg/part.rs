use crate::irc::{Command, IrcMessageRef};

/// Sent when a user leaves a channel.
#[derive(Clone, Debug)]
pub struct Part<'src> {
  /// Parted channel name.
  pub channel: &'src str,

  /// Login of the user.
  pub user: &'src str,
}

impl<'src> super::FromIrc<'src> for Part<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Part {
      return None;
    }

    Some(Part {
      channel: message.channel()?,
      user: message.prefix().and_then(|prefix| prefix.nick)?,
    })
  }
}

impl<'src> From<Part<'src>> for super::Message<'src> {
  fn from(msg: Part<'src>) -> Self {
    super::Message::Part(msg)
  }
}
