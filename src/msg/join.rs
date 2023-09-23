use crate::irc::{Command, IrcMessageRef};

/// Sent when a user joins a channel.
#[derive(Clone, Debug)]
pub struct Join<'src> {
  /// Joined channel name.
  pub channel: &'src str,

  /// Login of the user.
  pub user: &'src str,
}

impl<'src> super::FromIrc<'src> for Join<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Join {
      return None;
    }

    Some(Join {
      channel: message.channel()?,
      user: message.prefix().and_then(|prefix| prefix.nick)?,
    })
  }
}

impl<'src> From<Join<'src>> for super::Message<'src> {
  fn from(msg: Join<'src>) -> Self {
    super::Message::Join(msg)
  }
}
