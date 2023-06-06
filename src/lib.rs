use std::collections::HashMap;

#[derive(Clone)]
pub struct Message {
  raw: String,
  tags: Option<Tags<'static>>,
  prefix: Option<Prefix<'static>>,
  command: Command<'static>,
  channel: Option<&'static str>,
  params: Option<&'static str>,
}

impl Message {
  pub fn parse(src: impl Into<String>) -> Option<Self> {
    // rust-analyzer is not smart enough to infer `String` here...
    let raw: String = src.into();
    let remainder = &raw[..];

    let (tags, remainder) = parse_tags(remainder);
    let (prefix, remainder) = parse_prefix(remainder);
    let (command, remainder) = parse_command(remainder)?;
    let (channel, remainder) = parse_channel(remainder);
    let params = parse_params(remainder);

    Some(Self {
      raw,
      tags,
      prefix,
      command,
      channel,
      params,
    })
  }

  pub fn raw(&self) -> &str {
    &self.raw
  }

  pub fn tags(&self) -> Option<&Tags> {
    self.tags.as_ref()
  }

  pub fn prefix(&self) -> Option<Prefix<'_>> {
    self.prefix
  }

  pub fn command(&self) -> Command<'_> {
    self.command
  }

  pub fn channel(&self) -> Option<&str> {
    self.channel
  }

  pub fn params(&self) -> Option<&str> {
    self.params
  }
}

pub type Tags<'src> = HashMap<Tag<'src>, &'src str>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Command<'src> {
  Ping,
  Pong,
  /// Join channel
  Join,
  /// Leave channel
  Part,
  /// Twitch Private Message
  Privmsg,
  // Twitch extensions
  /// Send message to a single user
  Whisper,
  /// Purge a user's messages
  Clearchat,
  /// Single message removal
  Clearmsg,
  /// Sent upon successful authentication (PASS/NICK command)
  GlobalUserState,
  /// Channel starts or stops host mode
  HostTarget,
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
  RplWelcome,
  RplYourHost,
  RplCreated,
  RplMyInfo,
  RplNamReply,
  RplEndOfNames,
  RplMotd,
  RplMotdStart,
  RplEndOfMotd,
  /// Unknown command
  Unknown(&'src str),
}

/// `@a=a;b=b;c= :<rest>`
fn parse_tags(remainder: &str) -> (Option<Tags<'static>>, &str) {
  if let Some(remainder) = remainder.strip_prefix('@') {
    let mut tags = Tags::with_capacity(16);
    let mut key = (0, 0);
    let mut value = (0, 0);
    let mut end = 0;
    let bytes = remainder.as_bytes();
    for i in 0..bytes.len() {
      match unsafe { *bytes.get_unchecked(i) } {
        b' ' if unsafe { *bytes.get_unchecked(i + 1) } == b':' => {
          value.1 = i;
          if key.1 - key.0 > 0 {
            tags.insert(
              tag_from_str(unsafe { &*(&remainder[key.0..key.1] as *const _) }),
              unsafe { &*(&remainder[value.0..value.1] as *const _) },
            );
          }
          end = i;
          break;
        }
        b'=' => {
          key.1 = i;
          value.0 = i + 1;
          value.1 = i + 1;
        }
        b';' => {
          value.1 = i;

          tags.insert(
            tag_from_str(unsafe { &*(&remainder[key.0..key.1] as *const _) }),
            unsafe { &*(&remainder[value.0..value.1] as *const _) },
          );

          key.0 = i + 1;
          key.1 = i + 1;
        }
        _ => {}
      }
    }

    (Some(tags), &remainder[end + 1..])
  } else {
    (None, remainder)
  }
}

macro_rules! tags_def {
  (
    $tag:ident, $tag_from_str:ident;
    $($(#[$meta:meta])* $key:literal = $name:ident),*
  ) => {
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub enum $tag<'src> {
      $(
        $(#[$meta])*
        $name,
      )*
      Unknown(&'src str),
    }

    fn $tag_from_str(v: &str) -> Tag {
      match v {
        $($key => $tag::$name,)*
        v => $tag::Unknown(v),
      }
    }
  }
}

tags_def! {
  Tag, tag_from_str;
  "msg-id" = MsgId,
  "badges" = Badges,
  "badge-info" = BadgeInfo,
  "display-name" = DisplayName,
  "emote-only" = EmoteOnly,
  "emotes" = Emotes,
  "flags" = Flags,
  "id" = Id,
  "mod" = Mod,
  "room-id" = RoomId,
  "subscriber" = Subscriber,
  "tmi-sent-ts" = TmiSentTs,
  "turbo" = Turbo,
  "user-id" = UserId,
  "user-type" = UserType,
  "client-nonce" = ClientNonce,
  "first-msg" = FirstMsg,
  "reply-parent-display-name" = ReplyParentDisplayName,
  "reply-parent-msg-body" = ReplyParentMsgBody,
  "reply-parent-msg-id" = ReplyParentMsgId,
  "reply-parent-user-id" = ReplyParentUserId,
  "reply-parent-user-login" = ReplyParentUserLogin,
  "followers-only" = FollowersOnly,
  "r9k" = R9K,
  "rituals" = Rituals,
  "slow" = Slow,
  "subs-only" = SubsOnly,
  "msg-param-cumulative-months" = MsgParamCumulativeMonths,
  "msg-param-displayName" = MsgParamDisplayName,
  "msg-param-login" = MsgParamLogin,
  "msg-param-months" = MsgParamMonths,
  "msg-param-promo-gift-total" = MsgParamPromoGiftTotal,
  "msg-param-promo-name" = MsgParamPromoName,
  "msg-param-recipient-display-name" = MsgParamRecipientDisplayName,
  "msg-param-recipient-id" = MsgParamRecipientId,
  "msg-param-recipient-user-name" = MsgParamRecipientUserName,
  "msg-param-sender-login" = MsgParamSenderLogin,
  "msg-param-sender-name" = MsgParamSenderName,
  "msg-param-should-share-streak" = MsgParamShouldShareStreak,
  "msg-param-streak-months" = MsgParamStreakMonths,
  "msg-param-sub-plan" = MsgParamSubPlan,
  "msg-param-sub-plan-name" = MsgParamSubPlanName,
  "msg-param-viewerCount" = MsgParamViewerCount,
  "msg-param-ritual-name" = MsgParamRitualName,
  "msg-param-threshold" = MsgParamThreshold,
  "msg-param-gift-months" = MsgParamGiftMonths,
  "login" = Login,
  "system-msg" = SystemMsg,
  "emote-sets" = EmoteSets,
  "thread-id" = ThreadId,
  "message-id" = MessageId,
  "returning-chatter" = ReturningChatter,
  "color" = Color,
  "vip" = Vip,
  "target-user-id" = TargetUserId,
  "ban-duration" = BanDuration,
  "msg-param-multimonth-duration" = MsgParamMultimonthDuration,
  "msg-param-was-gifted" = MsgParamWasGifted,
  "msg-param-multimonth-tenure" = MsgParamMultimonthTenure,
  "sent-ts" = SentTs,
  "msg-param-origin-id" = MsgParamOriginId,
  "msg-param-fun-string" = MsgParamFunString,
  "msg-param-sender-count" = MsgParamSenderCount,
  "msg-param-profileImageURL" = MsgParamProfileImageUrl,
  "msg-param-mass-gift-count" = MsgParamMassGiftCount,
  "msg-param-gift-month-being-redeemed" = MsgParamGiftMonthBeingRedeemed,
  "msg-param-anon-gift" = MsgParamAnonGift
}

#[derive(Clone, Copy)]
pub struct Prefix<'src> {
  pub nick: Option<&'src str>,
  pub user: Option<&'src str>,
  pub host: &'src str,
}

/// `:nick!user@host <rest>`
fn parse_prefix(remainder: &str) -> (Option<Prefix<'static>>, &str) {
  if let Some(remainder) = remainder.strip_prefix(':') {
    // :host <rest>
    // :nick@host <rest>
    // :nick!user@host <rest>
    let bytes = remainder.as_bytes();

    let mut host_start = None;
    let mut nick = None;
    let mut nick_end = None;
    let mut user = None;
    for i in 0..bytes.len() {
      match unsafe { *bytes.get_unchecked(i) } {
        b' ' => {
          let host_range = match host_start {
            Some(host_start) => host_start..i,
            None => 0..i,
          };
          let host = unsafe { &*(&remainder[host_range] as *const _) };

          return (Some(Prefix { nick, user, host }), &remainder[i + 1..]);
        }
        b'@' => {
          host_start = Some(i + 1);
          if let Some(nick_end) = nick_end {
            user = Some(unsafe { &*(&remainder[nick_end + 1..i] as *const _) });
          } else {
            nick = Some(unsafe { &*(&remainder[..i] as *const _) });
          }
        }
        b'!' => {
          nick = Some(unsafe { &*(&remainder[..i] as *const _) });
          nick_end = Some(i);
        }
        _ => {}
      }
    }

    (None, remainder)
  } else {
    (None, remainder)
  }
}

/// `COMMAND <rest>`
///
/// Returns `None` if command is unknown *and* empty
fn parse_command(remainder: &str) -> Option<(Command<'static>, &str)> {
  let (cmd, remainder) = match remainder.split_once(' ') {
    Some(v) => v,
    None => (remainder, &remainder[remainder.len()..]),
  };

  use Command::*;
  let cmd = match cmd {
    "PING" => Ping,
    "PONG" => Pong,
    "JOIN" => Join,
    "PART" => Part,
    "PRIVMSG" => Privmsg,
    "WHISPER" => Whisper,
    "CLEARCHAT" => Clearchat,
    "CLEARMSG" => Clearmsg,
    "GLOBALUSERSTATE" => GlobalUserState,
    "HOSTTARGET" => HostTarget,
    "NOTICE" => Notice,
    "RECONNECT" => Reconnect,
    "ROOMSTATE" => RoomState,
    "USERNOTICE" => UserNotice,
    "USERSTATE" => UserState,
    "CAP" => Capability,
    "001" => RplWelcome,
    "002" => RplYourHost,
    "003" => RplCreated,
    "004" => RplMyInfo,
    "353" => RplNamReply,
    "366" => RplEndOfNames,
    "372" => RplMotd,
    "375" => RplMotdStart,
    "376" => RplEndOfMotd,
    other if !other.is_empty() => Unknown(unsafe { &*(cmd as *const _) }),
    _ => return None,
  };

  Some((cmd, remainder))
}

/// #channel <rest>
fn parse_channel(remainder: &str) -> (Option<&'static str>, &str) {
  if remainder.starts_with('#') {
    let (channel, remainder) = match remainder.split_once(' ') {
      Some(v) => v,
      None => (remainder, &remainder[remainder.len()..]),
    };

    // SAFETY: `channel` is a subslice of `base`.
    (Some(unsafe { &*(channel as *const _) }), remainder)
  } else {
    (None, remainder)
  }
}

fn parse_params(remainder: &str) -> Option<&'static str> {
  if !remainder.is_empty() {
    // SAFETY: `remainder` is a subslice of `base`.
    Some(unsafe { &*(remainder as *const _) })
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod parse {
    use super::*;

    #[test]
    fn tags() {
      let data = "@login=test;id=asdf :<rest>";

      let (tags, remainder) = parse_tags(data);
      assert_eq!(remainder, &data[20..]);
      let tags = tags.unwrap();
      assert_eq!(
        &tags,
        &[(Tag::Login, "test"), (Tag::Id, "asdf")]
          .into_iter()
          .collect()
      )
    }

    #[test]
    fn prefix() {
      let data = ":nick!user@host <rest>";

      let (prefix, remainder) = parse_prefix(data);
      assert_eq!(remainder, &data[16..]);
      let prefix = prefix.unwrap();
      assert_eq!(prefix.nick.unwrap(), "nick");
      assert_eq!(prefix.user.unwrap(), "user");
      assert_eq!(prefix.host, "host");
      assert_eq!(remainder, "<rest>");

      let data = ":nick@host <rest>";
      let (prefix, remainder) = parse_prefix(data);
      assert_eq!(remainder, &data[11..]);
      let prefix = prefix.unwrap();
      assert_eq!(prefix.nick.unwrap(), "nick");
      assert!(prefix.user.is_none());
      assert_eq!(prefix.host, "host");
      assert_eq!(remainder, "<rest>");

      let data = ":host <rest>";
      let (prefix, remainder) = parse_prefix(data);
      assert_eq!(remainder, &data[6..]);
      let prefix = prefix.unwrap();
      assert!(prefix.nick.is_none());
      assert!(prefix.user.is_none());
      assert_eq!(prefix.host, "host");
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn command() {
      let data = "PING <rest>";

      let (command, remainder) = parse_command(data).unwrap();
      assert_eq!(command, Command::Ping);
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn channel() {
      let data = "#channel <rest>";

      let (channel, remainder) = parse_channel(data);
      let channel = channel.unwrap();
      assert_eq!(channel, "#channel");
      assert_eq!(remainder, "<rest>");
    }

    #[test]
    fn params() {
      let data = ":param_a :param_b";

      let params = parse_params(data);
      let params = params.unwrap();
      assert_eq!(params, data)
    }
  }
}
