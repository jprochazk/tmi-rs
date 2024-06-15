use std::fmt::Display;

use crate::common::Span;

#[derive(Clone, Copy)]
pub(super) enum RawCommand {
  Ping,
  Pong,
  Join,
  Part,
  Privmsg,
  Whisper,
  Clearchat,
  Clearmsg,
  GlobalUserState,
  Notice,
  Reconnect,
  RoomState,
  UserNotice,
  UserState,
  Capability,
  RplWelcome,
  RplYourHost,
  RplCreated,
  RplMyInfo,
  RplNamReply,
  RplEndOfNames,
  RplMotd,
  RplMotdStart,
  RplEndOfMotd,
  Other(Span),
}

impl RawCommand {
  #[inline]
  pub(super) fn get<'src>(&self, src: &'src str) -> Command<'src> {
    match self {
      RawCommand::Ping => Command::Ping,
      RawCommand::Pong => Command::Pong,
      RawCommand::Join => Command::Join,
      RawCommand::Part => Command::Part,
      RawCommand::Privmsg => Command::Privmsg,
      RawCommand::Whisper => Command::Whisper,
      RawCommand::Clearchat => Command::ClearChat,
      RawCommand::Clearmsg => Command::ClearMsg,
      RawCommand::GlobalUserState => Command::GlobalUserState,
      RawCommand::Notice => Command::Notice,
      RawCommand::Reconnect => Command::Reconnect,
      RawCommand::RoomState => Command::RoomState,
      RawCommand::UserNotice => Command::UserNotice,
      RawCommand::UserState => Command::UserState,
      RawCommand::Capability => Command::Capability,
      RawCommand::RplWelcome => Command::RplWelcome,
      RawCommand::RplYourHost => Command::RplYourHost,
      RawCommand::RplCreated => Command::RplCreated,
      RawCommand::RplMyInfo => Command::RplMyInfo,
      RawCommand::RplNamReply => Command::RplNames,
      RawCommand::RplEndOfNames => Command::RplEndOfNames,
      RawCommand::RplMotd => Command::RplMotd,
      RawCommand::RplMotdStart => Command::RplMotdStart,
      RawCommand::RplEndOfMotd => Command::RplEndOfMotd,
      RawCommand::Other(span) => Command::Other(&src[*span]),
    }
  }
}

/// A Twitch IRC command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command<'src> {
  /// Ping the peer
  Ping,
  /// The peer's response to a [`Command::Ping`]
  Pong,
  /// Join a channel
  Join,
  /// Leave a channel
  Part,
  /// Send a message to a channel
  Privmsg,
  /// Send a private message to a user
  Whisper,
  /// Purge a user's messages in a channel
  ClearChat,
  /// Remove a single message
  ClearMsg,
  /// Sent upon successful authentication (PASS/NICK command)
  GlobalUserState,
  /// General notices from the server
  Notice,
  /// Rejoins channels after a restart
  Reconnect,
  /// Identifies the channel's chat settings
  RoomState,
  /// Announces Twitch-specific events to the channel
  UserNotice,
  /// Identifies a user's chat settings or properties
  UserState,
  /// Requesting an IRC capability
  Capability,
  // Numeric commands
  /// `001`
  RplWelcome,
  /// `002`
  RplYourHost,
  /// `003`
  RplCreated,
  /// `004`
  RplMyInfo,
  /// `353`
  RplNames,
  /// `366`
  RplEndOfNames,
  /// `372`
  RplMotd,
  /// `375`
  RplMotdStart,
  /// `376`
  RplEndOfMotd,
  /// Unknown command
  Other(&'src str),
}

impl<'src> Display for Command<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

impl<'src> Command<'src> {
  /// Get the string value of the [`Command`].
  pub fn as_str(&self) -> &'src str {
    use Command::*;
    match self {
      Ping => "PING",
      Pong => "PONG",
      Join => "JOIN",
      Part => "PART",
      Privmsg => "PRIVMSG",
      Whisper => "WHISPER",
      ClearChat => "CLEARCHAT",
      ClearMsg => "CLEARMSG",
      GlobalUserState => "GLOBALUSERSTATE",
      Notice => "NOTICE",
      Reconnect => "RECONNECT",
      RoomState => "ROOMSTATE",
      UserNotice => "USERNOTICE",
      UserState => "USERSTATE",
      Capability => "CAP",
      RplWelcome => "001",
      RplYourHost => "002",
      RplCreated => "003",
      RplMyInfo => "004",
      RplNames => "353",
      RplEndOfNames => "366",
      RplMotd => "372",
      RplMotdStart => "375",
      RplEndOfMotd => "376",
      Other(cmd) => cmd,
    }
  }
}

/// `COMMAND <rest>`
///
/// Returns `None` if command is unknown *and* empty
#[inline(always)]
pub(super) fn parse(src: &str, pos: &mut usize) -> Option<RawCommand> {
  let (end, next_pos) = match src[*pos..].find(' ') {
    Some(end) => {
      let end = *pos + end;
      (end, end + 1)
    }
    None => (src.len(), src.len()),
  };

  use RawCommand as C;
  let cmd = match &src[*pos..end] {
    "PING" => C::Ping,
    "PONG" => C::Pong,
    "JOIN" => C::Join,
    "PART" => C::Part,
    "PRIVMSG" => C::Privmsg,
    "WHISPER" => C::Whisper,
    "CLEARCHAT" => C::Clearchat,
    "CLEARMSG" => C::Clearmsg,
    "GLOBALUSERSTATE" => C::GlobalUserState,
    "NOTICE" => C::Notice,
    "RECONNECT" => C::Reconnect,
    "ROOMSTATE" => C::RoomState,
    "USERNOTICE" => C::UserNotice,
    "USERSTATE" => C::UserState,
    "CAP" => C::Capability,
    "001" => C::RplWelcome,
    "002" => C::RplYourHost,
    "003" => C::RplCreated,
    "004" => C::RplMyInfo,
    "353" => C::RplNamReply,
    "366" => C::RplEndOfNames,
    "372" => C::RplMotd,
    "375" => C::RplMotdStart,
    "376" => C::RplEndOfMotd,
    other if !other.is_empty() => C::Other(Span::from(*pos..end)),
    _ => return None,
  };

  *pos = next_pos;

  Some(cmd)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn command() {
    let data = "PING <rest>";
    let mut pos = 0;

    let command = parse(data, &mut pos).unwrap();
    assert_eq!(command.get(data), Command::Ping);
    assert_eq!(&data[pos..], "<rest>");
  }
}
