use crate::irc::{Command, IrcMessageRef, Tag};

/// Sent by TMI for various reasons to notify the client about something,
/// usually in response to invalid actions.
#[derive(Clone, Debug)]
pub struct Notice<'src> {
  /// Target channel name.
  ///
  /// This may be empty before successful login.
  pub channel: Option<&'src str>,

  /// Notice message.
  pub text: &'src str,

  /// Notice ID, see <https://dev.twitch.tv/docs/irc/msg-id/>.
  ///
  /// This will only be empty before successful login.
  pub id: Option<&'src str>,
}

impl<'src> super::FromIrc<'src> for Notice<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Notice {
      return None;
    }

    Some(Notice {
      channel: message.channel(),
      text: message.text()?,
      id: message.tag(Tag::MsgId),
    })
  }
}

impl<'src> From<Notice<'src>> for super::Message<'src> {
  fn from(msg: Notice<'src>) -> Self {
    super::Message::Notice(msg)
  }
}
