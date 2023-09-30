use crate::common::Channel;
use crate::irc::{Command, IrcMessageRef, Tag};

/// Sent by TMI for various reasons to notify the client about something,
/// usually in response to invalid actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice<'src> {
  channel: Option<Channel<'src>>,
  text: &'src str,
  id: Option<&'src str>,
}

generate_getters! {
  <'src> for Notice<'src> as self {
    /// Target channel name.
    ///
    /// This may be empty before successful login.
    channel -> Option<Channel<'_>>,

    /// Notice message.
    text -> &str,

    /// Notice ID, see <https://dev.twitch.tv/docs/irc/msg-id/>.
    ///
    /// This will only be empty before successful login.
    id -> Option<&str>,
  }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_notice_before_login() {
    assert_irc_snapshot!(Notice, ":tmi.twitch.tv NOTICE * :Improperly formatted auth");
  }

  #[test]
  fn parse_notice_basic() {
    assert_irc_snapshot!(Notice, "@msg-id=msg_banned :tmi.twitch.tv NOTICE #forsen :You are permanently banned from talking in forsen.");
  }
}
