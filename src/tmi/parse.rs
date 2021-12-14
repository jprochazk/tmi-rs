///! Twitch IRC message parser
///!
///! This *only* handles parsing.
///!
///! ```
///! match Message::parse(/* receive a message somehow */).unwrap() {
///!     Message::Privmsg(msg) => handle_privmsg(msg)
///! }
///! ```
use std::convert::{Into, TryFrom};

use chrono::{DateTime, Duration, Utc};
use thiserror::Error;
use twitch_getters::twitch_getters;

// TODO: there are still a bunch of String allocations which can be removed
use super::{irc, irc::DurationKind, util::UnsafeSlice};
#[derive(Error, Debug, PartialEq)]
pub enum Error {
  #[error("Invalid tag '{0}'")]
  InvalidTag(String),
  #[error("Invalid command '{0}'")]
  InvalidCommand(String),
  #[error("Invalid channel '{0}'")]
  InvalidChannel(String),
  #[error("Expected param '{0}'")]
  MissingParam(String),
  #[error("No parameters received")]
  EmptyParams,
  #[error("Expected nickname")]
  MissingNick(String),
  #[error("Invalid badge '{0}'")]
  InvalidBadge(String),
  #[error("Invalid emote '{0}'")]
  InvalidEmote(String),
  #[error("Invalid value '{1}' for tag '{0}'")]
  InvalidTagValue(String, String),
  #[error("Received a malformed message")]
  MalformedMessage,
  #[error(transparent)]
  Irc(#[from] irc::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub enum Message {
  Ping(self::Ping),
  Pong(self::Pong),
  Join(self::Join),
  Part(self::Part),
  Privmsg(self::Privmsg),
  Whisper(self::Whisper),
  Clearchat(self::Clearchat),
  Clearmsg(self::Clearmsg),
  GlobalUserState(self::GlobalUserState),
  HostTarget(self::HostTarget),
  Notice(self::Notice),
  Reconnect(self::Reconnect),
  RoomState(self::RoomState),
  UserNotice(self::UserNotice),
  UserState(self::UserState),
  Capability(self::Capability),
  Unknown(irc::Message),
}

impl Message {
  pub fn parse(data: impl Into<String>) -> Result<Message> {
    Self::try_from(irc::Message::parse(data)?)
  }
}

impl TryFrom<irc::Message> for Message {
  type Error = Error;
  fn try_from(msg: irc::Message) -> Result<Self> {
    Ok(match msg.cmd {
      irc::Command::Ping => Message::Ping(Ping::parse(msg)?),
      irc::Command::Pong => Message::Pong(Pong::parse(msg)?),
      irc::Command::Join => Message::Join(Join::parse(msg)?),
      irc::Command::Part => Message::Part(Part::parse(msg)?),
      irc::Command::Privmsg => Message::Privmsg(Privmsg::parse(msg)?),
      irc::Command::Whisper => Message::Whisper(Whisper::parse(msg)?),
      irc::Command::Clearchat => Message::Clearchat(Clearchat::parse(msg)?),
      irc::Command::Clearmsg => Message::Clearmsg(Clearmsg::parse(msg)?),
      irc::Command::GlobalUserState => Message::GlobalUserState(GlobalUserState::parse(msg)?),
      irc::Command::HostTarget => Message::HostTarget(HostTarget::parse(msg)?),
      irc::Command::Notice => Message::Notice(Notice::parse(msg)?),
      irc::Command::Reconnect => Message::Reconnect(Reconnect::parse(msg)?),
      irc::Command::RoomState => Message::RoomState(RoomState::parse(msg)?),
      irc::Command::UserNotice => Message::UserNotice(UserNotice::parse(msg)?),
      irc::Command::UserState => Message::UserState(UserState::parse(msg)?),
      irc::Command::Capability => Message::Capability(Capability::parse(msg)?),
      _ => Message::Unknown(msg),
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Ping {
  arg: Option<UnsafeSlice>,
  raw: irc::Message,
}

impl Ping {
  fn parse(value: irc::Message) -> Result<Ping> {
    Ok(Ping {
      arg: value
        .params
        .as_ref()
        .map(|v| v.raw().strip_prefix(':'))
        .flatten()
        .map(|v| v.into()),
      raw: value,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Pong {
  arg: Option<UnsafeSlice>,
  raw: irc::Message,
}

impl Pong {
  fn parse(value: irc::Message) -> Result<Pong> {
    Ok(Pong {
      arg: value
        .params
        .as_ref()
        .map(|v| v.raw().strip_prefix(':'))
        .flatten()
        .map(|v| v.into()),
      raw: value,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Join {
  channel: UnsafeSlice,
  nick: UnsafeSlice,
  raw: irc::Message,
}

impl Join {
  fn parse(value: irc::Message) -> Result<Self> {
    Ok(Join {
      channel: value
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      nick: match value
        .prefix
        .as_ref()
        .ok_or_else(|| Error::MissingParam("nick".into()))?
        .nick()
      {
        Some(nick) => nick.into(),
        None => return Err(Error::MissingParam("user".into())),
      },
      raw: value,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Part {
  channel: UnsafeSlice,
  nick: UnsafeSlice,
  raw: irc::Message,
}

impl Part {
  fn parse(value: irc::Message) -> Result<Self> {
    Ok(Part {
      channel: value
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      nick: match value
        .prefix
        .as_ref()
        .ok_or_else(|| Error::MissingParam("nick".into()))?
        .nick()
      {
        Some(nick) => nick.into(),
        None => return Err(Error::MissingParam("nick".into())),
      },
      raw: value,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct TwitchUser {
  /// The unique ID of the user - the `login` and `name` fields may
  /// arbitrarily change, but ID stays the same forever. For that reason,
  /// use this when identifying a user.
  id: UnsafeSlice,
  /// Refers to the user's 'login' name, which is usually just the lowercased
  /// version of `name`
  login: UnsafeSlice,
  /// Refers to the user's 'display' name, which should be used in user-facing
  /// contexts.
  pub name: String,
  badge_info: Option<UnsafeSlice>,
  badges: Option<UnsafeSlice>,
}

impl TwitchUser {
  pub fn has_badge(&self, badge: &str) -> bool {
    self
      .badges
      .as_ref()
      .map(|v| v.as_str().contains(badge))
      .unwrap_or(false)
  }
  pub fn is_mod(&self) -> bool {
    self.has_badge("moderator")
  }
  pub fn is_streamer(&self) -> bool {
    self.has_badge("broadcaster")
  }
  pub fn is_vip(&self) -> bool {
    self.has_badge("vip")
  }
}

/// If message starts with '\x01ACTION ' and ends with '\x01', then remove those
fn parse_message(msg: &str) -> (&str, bool) {
  msg
    .strip_prefix("\x01ACTION ")
    .and_then(|v| v.strip_suffix('\x01'))
    .map(|v| (v, true))
    .unwrap_or((msg, false))
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Privmsg {
  channel: UnsafeSlice,
  text: UnsafeSlice,
  pub user: TwitchUser,
  pub is_action: bool,
  pub bits: Option<i64>,
  color: Option<UnsafeSlice>,
  #[csv]
  emotes: UnsafeSlice,
  id: UnsafeSlice,
  room_id: UnsafeSlice,
  pub time: DateTime<Utc>,
  custom_reward_id: Option<UnsafeSlice>,
  raw: irc::Message,
}

impl Privmsg {
  fn parse(source: irc::Message) -> Result<Self> {
    let (text, is_action) = match source.params.as_ref() {
      Some(v) => parse_message(v.raw().trim_start().strip_prefix(':').ok_or(Error::MalformedMessage)?),
      None => ("", false),
    };
    Ok(Privmsg {
      channel: source
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      text: text.into(),
      user: TwitchUser {
        id: source.tags.require_raw("user-id")?,
        login: source
          .prefix
          .as_ref()
          .ok_or_else(|| Error::MissingParam("nick".into()))?
          .nick()
          .ok_or_else(|| Error::MissingParam("nick".into()))?
          .into(),
        name: source.tags.require_ns("display-name")?,
        badge_info: source.tags.get_raw("badge-info"),
        badges: source.tags.get_raw("badges"),
      },
      is_action,
      bits: source.tags.get_number("bits"),
      color: source.tags.get_raw("color"),
      emotes: source.tags.get_raw("emotes").unwrap_or_default(),
      id: source.tags.require_raw("id")?,
      room_id: source.tags.require_raw("room-id")?,
      time: source.tags.require_date("tmi-sent-ts")?,
      custom_reward_id: source.tags.get_raw("custom-reward-id"),
      raw: source,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Whisper {
  recipient: UnsafeSlice,
  thread_id: UnsafeSlice,
  pub user: TwitchUser,
  text: UnsafeSlice,
  pub is_action: bool,
  color: Option<UnsafeSlice>,
  #[csv]
  emotes: UnsafeSlice,
  id: UnsafeSlice,
  raw: irc::Message,
}

impl Whisper {
  fn parse(source: irc::Message) -> Result<Self> {
    let params = match source.params.as_ref() {
      Some(v) => v,
      None => return Err(Error::EmptyParams),
    };
    let (recipient, message) = match params.raw().split_once(' ') {
      Some((r, m)) => (r, m),
      None => return Err(Error::MissingParam("message".into())),
    };
    let message = match message.trim_start().strip_prefix(':') {
      Some(v) => v,
      None => return Err(Error::MalformedMessage),
    };
    let (text, is_action) = parse_message(message);

    Ok(Whisper {
      recipient: recipient.into(),
      thread_id: source.tags.require_raw("thread-id")?,
      user: TwitchUser {
        id: source.tags.require_raw("user-id")?,
        login: source
          .prefix
          .as_ref()
          .ok_or_else(|| Error::MissingParam("nick".into()))?
          .nick()
          .ok_or_else(|| Error::MissingParam("nick".into()))?
          .into(),
        name: source.tags.require_ns("display-name")?,
        badge_info: source.tags.get_raw("badge-info"),
        badges: source.tags.get_raw("badges"),
      },
      text: text.into(),
      is_action,
      color: source.tags.get_raw("color"),
      emotes: source.tags.get_raw("emotes").unwrap_or_default(),
      id: source.tags.require_raw("message-id")?,
      raw: source,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Clearchat {
  channel: UnsafeSlice,
  /// None = clear the entire chat
  target: Option<UnsafeSlice>,
  target_id: Option<UnsafeSlice>,
  pub time: DateTime<Utc>,
  /// None = permanent ban
  pub duration: Option<Duration>,
  raw: irc::Message,
}

impl Clearchat {
  fn parse(source: irc::Message) -> Result<Self> {
    let target = source
      .params
      .as_ref()
      .map(|v| v.raw())
      .map(|v| v.trim_start().strip_prefix(':').unwrap_or(v))
      .map(|v| v.into());

    Ok(Clearchat {
      channel: source
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      target,
      target_id: source.tags.get_raw("target-user-id"),
      time: source.tags.require_date("tmi-sent-ts")?,
      duration: source.tags.get_duration("ban-duration", DurationKind::Seconds),
      raw: source,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Clearmsg {
  channel: UnsafeSlice,
  login: UnsafeSlice,
  /// Deleted message text
  text: UnsafeSlice,
  target_msg_id: UnsafeSlice,
  raw: irc::Message,
}

impl Clearmsg {
  fn parse(source: irc::Message) -> Result<Self> {
    let text = match source
      .params
      .as_ref()
      .map(|v| v.raw())
      .map(|v| v.trim_start().strip_prefix(':').unwrap_or(v))
    {
      Some(v) => v.into(),
      None => return Err(Error::MalformedMessage),
    };
    Ok(Clearmsg {
      channel: source
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      login: source.tags.require_raw("login")?,
      text,
      target_msg_id: source.tags.require_raw("target-msg-id")?,
      raw: source,
    })
  }
}

/// Sent following a successful authentication
#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct GlobalUserState {
  user_id: UnsafeSlice,
  pub display_name: String,
  badge_info: Option<UnsafeSlice>,
  #[csv]
  badges: UnsafeSlice,
  color: Option<UnsafeSlice>,
  #[csv]
  emote_sets: UnsafeSlice,
  raw: irc::Message,
}

impl GlobalUserState {
  fn parse(source: irc::Message) -> Result<Self> {
    Ok(GlobalUserState {
      user_id: source.tags.require_raw("user-id")?,
      display_name: source.tags.require_ns("display-name")?,
      badge_info: source.tags.get_raw("badge-info"),
      badges: source.tags.get_raw("badges").unwrap_or_default(),
      color: source.tags.get_raw("color"),
      emote_sets: source.tags.get_raw("emote-sets").unwrap_or_default(),
      raw: source,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct HostTarget {
  hosting_channel: UnsafeSlice,
  /// None = stop hosting
  target_channel: Option<UnsafeSlice>,
  pub viewer_count: Option<i64>,
  raw: irc::Message,
}

impl HostTarget {
  fn parse(source: irc::Message) -> Result<Self> {
    let (target_channel, viewer_count) = match source
      .params
      .as_ref()
      .map(|v| v.raw())
      .map(|v| v.trim_start().strip_prefix(':').unwrap_or(v))
      .map(|v| match v.split_once(' ') {
        Some((l, r)) => (
          if !l.is_empty() && l != "-" {
            Some(l.into())
          } else {
            None
          },
          if !r.is_empty() { r.parse().ok() } else { None },
        ),
        None => (None, None),
      }) {
      Some(v) => v,
      None => return Err(Error::MalformedMessage),
    };
    Ok(HostTarget {
      hosting_channel: source
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      target_channel,
      viewer_count,
      raw: source,
    })
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoticeId {
  /// <user> is already banned in this channel.
  AlreadyBanned,
  /// This room is not in emote-only mode.
  AlreadyEmoteOnlyOff,
  /// This room is already in emote-only mode.
  AlreadyEmoteOnlyOn,
  /// This room is not in r9k mode.
  AlreadyR9kOff,
  /// This room is already in r9k mode.
  AlreadyR9kOn,
  /// This room is not in subscribers-only mode.
  AlreadySubsOff,
  /// This room is already in subscribers-only mode.
  AlreadySubsOn,
  /// You cannot ban admin <user>. Please email support@twitch.tv if an admin
  /// is being abusive.
  BadBanAdmin,
  /// You cannot ban anonymous users.
  BadBanAnon,
  /// You cannot ban the broadcaster.
  BadBanBroadcaster,
  /// You cannot ban global moderator <user>. Please email support@twitch.tv
  /// if a global moderator is being abusive.
  BadBanGlobalMod,
  /// You cannot ban moderator <user> unless you are the owner of this
  /// channel.
  BadBanMod,
  /// You cannot ban yourself.
  BadBanSelf,
  /// You cannot ban a staff <user>. Please email support@twitch.tv if a staff
  /// member is being abusive.
  BadBanStaff,
  /// Failed to start commercial.
  BadCommercialError,
  /// You cannot delete the broadcaster's messages.
  BadDeleteMessageBroadcaster,
  /// You cannot delete messages from another moderator <user>.
  BadDeleteMessageMod,
  /// There was a problem hosting <channel>. Please try again in a minute.
  BadHostError,
  /// This channel is already hosting <channel>.
  BadHostHosting,
  /// Host target cannot be changed more than <number> times every half hour.
  BadHostRateExceeded,
  /// This channel is unable to be hosted.
  BadHostRejected,
  /// A channel cannot host itself.
  BadHostSelf,
  /// Sorry, /marker is not available through this client.
  BadMarkerClient,
  /// <user> is banned in this channel. You must unban this user before
  /// granting mod status.
  BadModBanned,
  /// <user> is already a moderator of this channel.
  BadModMod,
  /// You cannot set slow delay to more than <number> seconds.
  BadSlowDuration,
  /// You cannot timeout admin <user>. Please email support@twitch.tv if an
  /// admin is being abusive.
  BadTimeoutAdmin,
  /// You cannot timeout anonymous users.
  BadTimeoutAnon,
  /// You cannot timeout the broadcaster.
  BadTimeoutBroadcaster,
  /// You cannot time a user out for more than <seconds>.
  BadTimeoutDuration,
  /// You cannot timeout global moderator <user>. Please email
  /// support@twitch.tv if a global moderator is being abusive.
  BadTimeoutGlobalMod,
  /// You cannot timeout moderator <user> unless you are the owner of this
  /// channel.
  BadTimeoutMod,
  /// You cannot timeout yourself.
  BadTimeoutSelf,
  /// You cannot timeout staff <user>. Please email support@twitch.tv if a
  /// staff member is being abusive.
  BadTimeoutStaff,
  /// <user> is not banned from this channel.
  BadUnbanNoBan,
  /// There was a problem exiting host mode. Please try again in a minute.
  BadUnhostError,
  /// <user> is not a moderator of this channel.
  BadUnmodMod,
  /// <user> is now banned from this channel.
  BanSuccess,
  /// Commands available to you in this room (use /help <command> for
  /// details): <list of commands>
  CmdsAvailable,
  /// Your color has been changed.
  ColorChanged,
  /// Initiating <number> second commercial break. Keep in mind that your
  /// stream is still live and not everyone will get a commercial.
  CommercialSuccess,
  /// The message from <user> is now deleted.
  DeleteMessageSuccess,
  /// This room is no longer in emote-only mode.
  EmoteOnlyOff,
  /// This room is now in emote-only mode.
  EmoteOnlyOn,
  /// A user has extended their subscription.
  ExtendSub,
  /// This room is no longer in followers-only mode.Note: The followers tags
  /// are broadcast to a channel when a moderator makes changes.
  FollowersOff,
  /// This room is now in <duration> followers-only mode.Examples: “This room
  /// is now in 2 week followers-only mode.” or “This room is now in 1 minute
  /// followers-only mode.”
  FollowersOn,
  /// This room is now in followers-only mode.
  FollowersOnzero,
  /// Exited host mode.
  HostOff,
  /// Now hosting <channel>.
  HostOn,
  /// <user> is now hosting you.
  HostSuccess,
  /// <user> is now hosting you for up to <number> viewers.
  HostSuccessViewers,
  /// <channel> has gone offline. Exiting host mode.
  HostTargetWentOffline,
  /// <number> host commands remaining this half hour.
  HostsRemaining,
  /// Invalid username: <user>
  InvalidUser,
  /// You have added <user> as a moderator of this channel.
  ModSuccess,
  /// You are permanently banned from talking in <channel>.
  MsgBanned,
  /// Your message was not sent because it contained too many characters that
  /// could not be processed. If you believe this is an error, rephrase and
  /// try again.
  MsgBadCharacters,
  /// Your message was not sent because your account is not in good standing
  /// in this channel.
  MsgChannelBlocked,
  /// This channel has been suspended.
  MsgChannelSuspended,
  /// Your message was not sent because it is identical to the previous one
  /// you sent, less than 30 seconds ago.
  MsgDuplicate,
  /// This room is in emote only mode. You can find your currently available
  /// emoticons using the smiley in the chat text area.
  MsgEmoteonly,
  /// You must use Facebook Connect to send messages to this channel. You can
  /// see Facebook Connect in your Twitch settings under the connections tab.
  MsgFacebook,
  /// This room is in <duration> followers-only mode. Follow <channel> to join
  /// the community!Note: These msg_followers tags are kickbacks to a user who
  /// does not meet the criteria; that is, does not follow or has not followed
  /// long enough.
  MsgFollowersonly,
  /// This room is in <duration1> followers-only mode. You have been following
  /// for <duration2>. Continue following to chat!
  MsgFollowersonlyFollowed,
  /// This room is in followers-only mode. Follow <channel> to join the
  /// community!
  MsgFollowersonlyZero,
  /// This room is in r9k mode and the message you attempted to send is not
  /// unique.
  MsgR9k,
  /// Your message was not sent because you are sending messages too quickly.
  MsgRatelimit,
  /// Hey! Your message is being checked by mods and has not been sent.
  MsgRejected,
  /// Your message wasn't posted due to conflicts with the channel's
  /// moderation settings.
  MsgRejectedMandatory,
  /// The room was not found.
  MsgRoomNotFound,
  /// This room is in slow mode and you are sending messages too quickly. You
  /// will be able to talk again in <number> seconds.
  MsgSlowmode,
  /// This room is in subscribers only mode. To talk, purchase a channel subscription at https://www.twitch.tv/products/<broadcaster login name>/ticket?ref=subscriber_only_mode_chat.
  MsgSubsonly,
  /// Your account has been suspended.
  MsgSuspended,
  /// You are banned from talking in <channel> for <number> more seconds.
  MsgTimedout,
  /// This room requires a verified email address to chat. Please verify your email at https://www.twitch.tv/settings/profile.
  MsgVerifiedEmail,
  /// No help available.
  NoHelp,
  /// There are no moderators of this channel.
  NoMods,
  /// No channel is currently being hosted.
  NotHosting,
  /// You don’t have permission to perform that action.
  NoPermission,
  /// This room is no longer in r9k mode.
  R9kOff,
  /// This room is now in r9k mode.
  R9kOn,
  /// You already have a raid in progress.
  RaidErrorAlreadyRaiding,
  /// You cannot raid this channel.
  RaidErrorForbidden,
  /// A channel cannot raid itself.
  RaidErrorSelf,
  /// Sorry, you have more viewers than the maximum currently supported by
  /// raids right now.
  RaidErrorTooManyViewers,
  /// There was a problem raiding <channel>. Please try again in a minute.
  RaidErrorUnexpected,
  /// This channel is intended for mature audiences.
  RaidNoticeMature,
  /// This channel has follower or subscriber only chat.
  RaidNoticeRestrictedChat,
  /// The moderators of this channel are: <list of users>
  RoomMods,
  /// This room is no longer in slow mode.
  SlowOff,
  /// This room is now in slow mode. You may send messages every <number>
  /// seconds.
  SlowOn,
  /// This room is no longer in subscribers-only mode.
  SubsOff,
  /// This room is now in subscribers-only mode.
  SubsOn,
  /// <user> is not timed out from this channel.
  TimeoutNoTimeout,
  /// <user> has been timed out for <duration> seconds.
  TimeoutSuccess,
  /// The community has closed channel <channel> due to Terms of Service
  /// violations.
  TosBan,
  /// Only turbo users can specify an arbitrary hex color. Use one of the
  /// following instead: <list of colors>.
  TurboOnlyColor,
  /// <user> is no longer banned from this channel.
  UnbanSuccess,
  /// You have removed <user> as a moderator of this channel.
  UnmodSuccess,
  /// You do not have an active raid.
  UnraidErrorNoActiveRaid,
  /// There was a problem stopping the raid. Please try again in a minute.
  UnraidErrorUnexpected,
  /// The raid has been cancelled.
  UnraidSuccess,
  /// Unrecognized command: <command>
  UnrecognizedCmd,
  /// The command <command> cannot be used in a chatroom.
  UnsupportedChatroomsCmd,
  /// <user> is permanently banned. Use "/unban" to remove a ban.
  UntimeoutBanned,
  /// <user> is no longer timed out in this channel.
  UntimeoutSuccess,
  /// Usage: “/ban <username> [reason]” Permanently prevent a user from
  /// chatting. Reason is optional and will be shown to the target and other
  /// moderators. Use “/unban” to remove a ban.
  UsageBan,
  /// Usage: “/clear”Clear chat history for all users in this room.
  UsageClear,
  /// Usage: “/color” <color>Change your username color. Color must be in hex
  /// (#000000) or one of the following: Blue, BlueViolet, CadetBlue,
  /// Chocolate, Coral, DodgerBlue, Firebrick, GoldenRod, Green, HotPink,
  /// OrangeRed, Red, SeaGreen, SpringGreen, YellowGreen.
  UsageColor,
  /// Usage: “/commercial [length]”Triggers a commercial. Length (optional)
  /// must be a positive number of seconds.
  UsageCommercial,
  /// Usage: “/disconnect”Reconnects to chat.
  UsageDisconnect,
  /// Usage: /emoteonlyoff”Disables emote-only mode.
  UsageEmoteOnlyOff,
  /// Usage: “/emoteonly”Enables emote-only mode (only emoticons may be used
  /// in chat). Use /emoteonlyoff to disable.
  UsageEmoteOnlyOn,
  /// Usage: /followersoff”Disables followers-only mode.
  UsageFollowersOff,
  /// Usage: “/followersEnables followers-only mode (only users who have
  /// followed for “duration” may chat). Examples: “30m”, “1 week”, “5 days 12
  /// hours”. Must be less than 3 months.
  UsageFollowersOn,
  /// Usage: “/help”Lists the commands available to you in this room.
  UsageHelp,
  /// Usage: “/host <channel>”Host another channel. Use “/unhost” to unset
  /// host mode.
  UsageHost,
  /// Usage: “/marker <optional comment>”Adds a stream marker (with an
  /// optional comment, max 140 characters) at the current timestamp. You can
  /// use markers in the Highlighter for easier editing.
  UsageMarker,
  /// Usage: “/me <message>”Send an “emote” message in the third person.
  UsageMe,
  /// Usage: “/mod <username>”Grant mod status to a user. Use “/mods” to list
  /// the moderators of this channel.
  UsageMod,
  /// Usage: “/mods”Lists the moderators of this channel.
  UsageMods,
  /// Usage: “/r9kbetaoff”Disables r9k mode.
  UsageR9kOff,
  /// Usage: “/r9kbeta”Enables r9k mode.Use “/r9kbetaoff“ to disable.
  UsageR9kOn,
  /// Usage: “/raid <channel>”Raid another channel.Use “/unraid” to cancel the
  /// Raid.
  UsageRaid,
  /// Usage: “/slowoff”Disables slow mode.
  UsageSlowOff,
  /// Usage: “/slow” [duration]Enables slow mode (limit how often users may
  /// send messages). Duration (optional, default=<number>) must be a positive
  /// integer number of seconds.Use “/slowoff” to disable.
  UsageSlowOn,
  /// Usage: “/subscribersoff”Disables subscribers-only mode.
  UsageSubsOff,
  /// Usage: “/subscribers”Enables subscribers-only mode (only subscribers may
  /// chat in this channel).Use “/subscribersoff” to disable.
  UsageSubsOn,
  /// Usage: “/timeout <username> [duration][time unit] [reason]"Temporarily
  /// prevent a user from chatting. Duration (optional, default=10 minutes)
  /// must be a positive integer; time unit (optional, default=s) must be one
  /// of s, m, h, d, w; maximum duration is 2 weeks. Combinations like 1d2h
  /// are also allowed. Reason is optional and will be shown to the target
  /// user and other moderators.Use “untimeout” to remove a timeout.
  UsageTimeout,
  /// Usage: “/unban <username>”Removes a ban on a user.
  UsageUnban,
  /// Usage: “/unhost”Stop hosting another channel.
  UsageUnhost,
  /// Usage: “/unmod <username>”Revoke mod status from a user. Use “/mods” to
  /// list the moderators of this channel.
  UsageUnmod,
  /// Usage: “/unraid”Cancel the Raid.
  UsageUnraid,
  /// Usage: “/untimeout <username>”Removes a timeout on a user.
  UsageUntimeout,
  /// You have been banned from sending whispers.
  WhisperBanned,
  /// That user has been banned from receiving whispers.
  WhisperBannedRecipient,
  /// Usage: <login> <message>
  WhisperInvalidArgs,
  /// No user matching that login.
  WhisperInvalidLogin,
  /// You cannot whisper to yourself.
  WhisperInvalidSelf,
  /// You are sending whispers too fast. Try again in a minute.
  WhisperLimitPerMin,
  /// You are sending whispers too fast. Try again in a second.
  WhisperLimitPerSec,
  /// Your settings prevent you from sending this whisper.
  WhisperRestricted,
  /// That user's settings prevent them from receiving this whisper.
  WhisperRestrictedRecipient,
}

impl NoticeId {
  fn parse(value: &str) -> Result<NoticeId> {
    match value {
      "already_banned" => Ok(NoticeId::AlreadyBanned),
      "already_emote_only_off" => Ok(NoticeId::AlreadyEmoteOnlyOff),
      "already_emote_only_on" => Ok(NoticeId::AlreadyEmoteOnlyOn),
      "already_r9k_off" => Ok(NoticeId::AlreadyR9kOff),
      "already_r9k_on" => Ok(NoticeId::AlreadyR9kOn),
      "already_subs_off" => Ok(NoticeId::AlreadySubsOff),
      "already_subs_on" => Ok(NoticeId::AlreadySubsOn),
      "bad_ban_admin" => Ok(NoticeId::BadBanAdmin),
      "bad_ban_anon" => Ok(NoticeId::BadBanAnon),
      "bad_ban_broadcaster" => Ok(NoticeId::BadBanBroadcaster),
      "bad_ban_global_mod" => Ok(NoticeId::BadBanGlobalMod),
      "bad_ban_mod" => Ok(NoticeId::BadBanMod),
      "bad_ban_self" => Ok(NoticeId::BadBanSelf),
      "bad_ban_staff" => Ok(NoticeId::BadBanStaff),
      "bad_commercial_error" => Ok(NoticeId::BadCommercialError),
      "bad_delete_message_broadcaster" => Ok(NoticeId::BadDeleteMessageBroadcaster),
      "bad_delete_message_mod" => Ok(NoticeId::BadDeleteMessageMod),
      "bad_host_error" => Ok(NoticeId::BadHostError),
      "bad_host_hosting" => Ok(NoticeId::BadHostHosting),
      "bad_host_rate_exceeded" => Ok(NoticeId::BadHostRateExceeded),
      "bad_host_rejected" => Ok(NoticeId::BadHostRejected),
      "bad_host_self" => Ok(NoticeId::BadHostSelf),
      "bad_marker_client" => Ok(NoticeId::BadMarkerClient),
      "bad_mod_banned" => Ok(NoticeId::BadModBanned),
      "bad_mod_mod" => Ok(NoticeId::BadModMod),
      "bad_slow_duration" => Ok(NoticeId::BadSlowDuration),
      "bad_timeout_admin" => Ok(NoticeId::BadTimeoutAdmin),
      "bad_timeout_anon" => Ok(NoticeId::BadTimeoutAnon),
      "bad_timeout_broadcaster" => Ok(NoticeId::BadTimeoutBroadcaster),
      "bad_timeout_duration" => Ok(NoticeId::BadTimeoutDuration),
      "bad_timeout_global_mod" => Ok(NoticeId::BadTimeoutGlobalMod),
      "bad_timeout_mod" => Ok(NoticeId::BadTimeoutMod),
      "bad_timeout_self" => Ok(NoticeId::BadTimeoutSelf),
      "bad_timeout_staff" => Ok(NoticeId::BadTimeoutStaff),
      "bad_unban_no_ban" => Ok(NoticeId::BadUnbanNoBan),
      "bad_unhost_error" => Ok(NoticeId::BadUnhostError),
      "bad_unmod_mod" => Ok(NoticeId::BadUnmodMod),
      "ban_success" => Ok(NoticeId::BanSuccess),
      "cmds_available" => Ok(NoticeId::CmdsAvailable),
      "color_changed" => Ok(NoticeId::ColorChanged),
      "commercial_success" => Ok(NoticeId::CommercialSuccess),
      "delete_message_success" => Ok(NoticeId::DeleteMessageSuccess),
      "emote_only_off" => Ok(NoticeId::EmoteOnlyOff),
      "emote_only_on" => Ok(NoticeId::EmoteOnlyOn),
      "extendsub" => Ok(NoticeId::ExtendSub),
      "followers_off" => Ok(NoticeId::FollowersOff),
      "followers_on" => Ok(NoticeId::FollowersOn),
      "followers_onzero" => Ok(NoticeId::FollowersOnzero),
      "host_off" => Ok(NoticeId::HostOff),
      "host_on" => Ok(NoticeId::HostOn),
      "host_success" => Ok(NoticeId::HostSuccess),
      "host_success_viewers" => Ok(NoticeId::HostSuccessViewers),
      "host_target_went_offline" => Ok(NoticeId::HostTargetWentOffline),
      "hosts_remaining" => Ok(NoticeId::HostsRemaining),
      "invalid_user" => Ok(NoticeId::InvalidUser),
      "mod_success" => Ok(NoticeId::ModSuccess),
      "msg_banned" => Ok(NoticeId::MsgBanned),
      "msg_bad_characters" => Ok(NoticeId::MsgBadCharacters),
      "msg_channel_blocked" => Ok(NoticeId::MsgChannelBlocked),
      "msg_channel_suspended" => Ok(NoticeId::MsgChannelSuspended),
      "msg_duplicate" => Ok(NoticeId::MsgDuplicate),
      "msg_emoteonly" => Ok(NoticeId::MsgEmoteonly),
      "msg_facebook" => Ok(NoticeId::MsgFacebook),
      "msg_followersonly" => Ok(NoticeId::MsgFollowersonly),
      "msg_followersonly_followed" => Ok(NoticeId::MsgFollowersonlyFollowed),
      "msg_followersonly_zero" => Ok(NoticeId::MsgFollowersonlyZero),
      "msg_r9k" => Ok(NoticeId::MsgR9k),
      "msg_ratelimit" => Ok(NoticeId::MsgRatelimit),
      "msg_rejected" => Ok(NoticeId::MsgRejected),
      "msg_rejected_mandatory" => Ok(NoticeId::MsgRejectedMandatory),
      "msg_room_not_found" => Ok(NoticeId::MsgRoomNotFound),
      "msg_slowmode" => Ok(NoticeId::MsgSlowmode),
      "msg_subsonly" => Ok(NoticeId::MsgSubsonly),
      "msg_suspended" => Ok(NoticeId::MsgSuspended),
      "msg_timedout" => Ok(NoticeId::MsgTimedout),
      "msg_verified_email" => Ok(NoticeId::MsgVerifiedEmail),
      "no_help" => Ok(NoticeId::NoHelp),
      "no_mods" => Ok(NoticeId::NoMods),
      "not_hosting" => Ok(NoticeId::NotHosting),
      "no_permission" => Ok(NoticeId::NoPermission),
      "r9k_off" => Ok(NoticeId::R9kOff),
      "r9k_on" => Ok(NoticeId::R9kOn),
      "raid_error_already_raiding" => Ok(NoticeId::RaidErrorAlreadyRaiding),
      "raid_error_forbidden" => Ok(NoticeId::RaidErrorForbidden),
      "raid_error_self" => Ok(NoticeId::RaidErrorSelf),
      "raid_error_too_many_viewers" => Ok(NoticeId::RaidErrorTooManyViewers),
      "raid_error_unexpected" => Ok(NoticeId::RaidErrorUnexpected),
      "raid_notice_mature" => Ok(NoticeId::RaidNoticeMature),
      "raid_notice_restricted_chat" => Ok(NoticeId::RaidNoticeRestrictedChat),
      "room_mods" => Ok(NoticeId::RoomMods),
      "slow_off" => Ok(NoticeId::SlowOff),
      "slow_on" => Ok(NoticeId::SlowOn),
      "subs_off" => Ok(NoticeId::SubsOff),
      "subs_on" => Ok(NoticeId::SubsOn),
      "timeout_no_timeout" => Ok(NoticeId::TimeoutNoTimeout),
      "timeout_success" => Ok(NoticeId::TimeoutSuccess),
      "tos_ban" => Ok(NoticeId::TosBan),
      "turbo_only_color" => Ok(NoticeId::TurboOnlyColor),
      "unban_success" => Ok(NoticeId::UnbanSuccess),
      "unmod_success" => Ok(NoticeId::UnmodSuccess),
      "unraid_error_no_active_raid" => Ok(NoticeId::UnraidErrorNoActiveRaid),
      "unraid_error_unexpected" => Ok(NoticeId::UnraidErrorUnexpected),
      "unraid_success" => Ok(NoticeId::UnraidSuccess),
      "unrecognized_cmd" => Ok(NoticeId::UnrecognizedCmd),
      "unsupported_chatrooms_cmd" => Ok(NoticeId::UnsupportedChatroomsCmd),
      "untimeout_banned" => Ok(NoticeId::UntimeoutBanned),
      "untimeout_success" => Ok(NoticeId::UntimeoutSuccess),
      "usage_ban" => Ok(NoticeId::UsageBan),
      "usage_clear" => Ok(NoticeId::UsageClear),
      "usage_color" => Ok(NoticeId::UsageColor),
      "usage_commercial" => Ok(NoticeId::UsageCommercial),
      "usage_disconnect" => Ok(NoticeId::UsageDisconnect),
      "usage_emote_only_off" => Ok(NoticeId::UsageEmoteOnlyOff),
      "usage_emote_only_on" => Ok(NoticeId::UsageEmoteOnlyOn),
      "usage_followers_off" => Ok(NoticeId::UsageFollowersOff),
      "usage_followers_on" => Ok(NoticeId::UsageFollowersOn),
      "usage_help" => Ok(NoticeId::UsageHelp),
      "usage_host" => Ok(NoticeId::UsageHost),
      "usage_marker" => Ok(NoticeId::UsageMarker),
      "usage_me" => Ok(NoticeId::UsageMe),
      "usage_mod" => Ok(NoticeId::UsageMod),
      "usage_mods" => Ok(NoticeId::UsageMods),
      "usage_r9k_off" => Ok(NoticeId::UsageR9kOff),
      "usage_r9k_on" => Ok(NoticeId::UsageR9kOn),
      "usage_raid" => Ok(NoticeId::UsageRaid),
      "usage_slow_off" => Ok(NoticeId::UsageSlowOff),
      "usage_slow_on" => Ok(NoticeId::UsageSlowOn),
      "usage_subs_off" => Ok(NoticeId::UsageSubsOff),
      "usage_subs_on" => Ok(NoticeId::UsageSubsOn),
      "usage_timeout" => Ok(NoticeId::UsageTimeout),
      "usage_unban" => Ok(NoticeId::UsageUnban),
      "usage_unhost" => Ok(NoticeId::UsageUnhost),
      "usage_unmod" => Ok(NoticeId::UsageUnmod),
      "usage_unraid" => Ok(NoticeId::UsageUnraid),
      "usage_untimeout" => Ok(NoticeId::UsageUntimeout),
      "whisper_banned" => Ok(NoticeId::WhisperBanned),
      "whisper_banned_recipient" => Ok(NoticeId::WhisperBannedRecipient),
      "whisper_invalid_args" => Ok(NoticeId::WhisperInvalidArgs),
      "whisper_invalid_login" => Ok(NoticeId::WhisperInvalidLogin),
      "whisper_invalid_self" => Ok(NoticeId::WhisperInvalidSelf),
      "whisper_limit_per_min" => Ok(NoticeId::WhisperLimitPerMin),
      "whisper_limit_per_sec" => Ok(NoticeId::WhisperLimitPerSec),
      "whisper_restricted" => Ok(NoticeId::WhisperRestricted),
      "whisper_restricted_recipient" => Ok(NoticeId::WhisperRestrictedRecipient),
      _ => Err(Error::InvalidTagValue("msg-id".into(), value.into())),
    }
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Notice {
  pub id: Option<NoticeId>,
  channel: Option<UnsafeSlice>,
  message: UnsafeSlice,
  raw: irc::Message,
}

impl Notice {
  fn parse(source: irc::Message) -> Result<Self> {
    // if id.is_some() => @msg-id=some_id :tmi.twitch.tv NOTICE #forsen
    // :SOME_MESSAGE if id.is_none() => :tmi.twitch.tv NOTICE *
    // :SOME_MESSAGE                in this case we skip this ^
    let (id, message) = match source.tags.get_raw("msg-id") {
      Some(v) => (
        Some(NoticeId::parse(v.as_ref())?),
        source
          .params
          .as_ref()
          .map(|v| v.raw())
          .map(|v| v.trim_start().strip_prefix(':').unwrap_or(v))
          .map(|v| v.into())
          .ok_or(Error::MalformedMessage)?,
      ),
      None => (
        None,
        source
          .params
          .as_ref()
          .map(|v| v.raw())
          .map(|v| {
            v.strip_prefix('*')
              .unwrap_or(v)
              .trim_start()
              .strip_prefix(':')
              .unwrap_or(v)
          })
          .map(|v| v.into())
          .ok_or(Error::MalformedMessage)?,
      ),
    };
    Ok(Notice {
      id,
      channel: source.channel().map(|v| v.into()),
      message,
      raw: source,
    })
  }
}

#[derive(Debug, PartialEq)]
pub struct Reconnect {
  raw: irc::Message,
}

impl Reconnect {
  fn parse(source: irc::Message) -> Result<Self> {
    Ok(Reconnect { raw: source })
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FollowerOnlyMode {
  /// Follower-only mode disabled
  Disabled,
  /// Only followers can chat
  All,
  /// Must follow for at least `min` minutes
  Duration { min: Duration },
}

impl FollowerOnlyMode {
  fn parse(value: i64) -> FollowerOnlyMode {
    match value {
      n if n < 0 => FollowerOnlyMode::Disabled,
      n if n == 0 => FollowerOnlyMode::All,
      // n > 0
      n => FollowerOnlyMode::Duration {
        min: Duration::minutes(n),
      },
    }
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct RoomState {
  channel: UnsafeSlice,
  /// Only Twitch emotes are allowed in chat
  pub emote_only: Option<bool>,
  /// See `FollowerOnlyMode` for more info
  pub followers_only: Option<FollowerOnlyMode>,
  /// R9K - messages over 9 characters must be unique
  pub r9k: Option<bool>,
  /// Slow mode - value is how many seconds a chatter must wait inbetween
  /// messages
  ///
  /// Does not apply to elevated users (moderator+)
  pub slow: Option<i64>,
  /// Only users which are subscribed to the channel may chat
  pub subs_only: Option<bool>,
  /// Whether or not this RoomState is a partial update
  ///
  /// If you join a room, this will be false, and all fields will be `Some`.
  ///
  /// If someone sets the room to a different state, this will be true,
  /// and only the changed state will be `Some`, rest will be `None`.
  pub is_update: bool,
  raw: irc::Message,
}

impl RoomState {
  fn parse(source: irc::Message) -> Result<Self> {
    let channel = source
      .channel()
      .ok_or_else(|| Error::MissingParam("channel".into()))?
      .into();
    let emote_only = source.tags.get_bool("emote-only");
    let followers_only = source.tags.get_number("followers-only").map(FollowerOnlyMode::parse);
    let r9k = source.tags.get_bool("r9k");
    let slow = source.tags.get_number("slow");
    let subs_only = source.tags.get_bool("subs-only");
    Ok(RoomState {
      channel,
      emote_only,
      followers_only,
      r9k,
      slow,
      subs_only,
      is_update: emote_only.is_none()
        || followers_only.is_none()
        || r9k.is_none()
        || slow.is_none()
        || subs_only.is_none(),
      raw: source,
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct UserNoticeBase {
  channel: UnsafeSlice,
  text: Option<UnsafeSlice>,
  pub user: TwitchUser,
  color: Option<UnsafeSlice>,
  #[csv]
  emotes: UnsafeSlice,
  id: UnsafeSlice,
  room_id: UnsafeSlice,
  pub system_msg: String,
  pub time: DateTime<Utc>,
  raw: irc::Message,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Sub {
  pub base: UserNoticeBase,
  pub cumulative_months: i64,
  pub should_share_streak: bool,
  pub streak_months: i64,
  sub_plan: UnsafeSlice,
  pub sub_plan_name: String,
  pub is_resub: bool,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct SubExtension {
  pub base: UserNoticeBase,
  pub cumulative_months: i64,
  sub_plan: UnsafeSlice,
  pub sub_plan_name: Option<String>,
  pub benefit_end_month: i64,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct SubGift {
  pub base: UserNoticeBase,
  pub cumulative_months: i64,
  pub recipient_display_name: String,
  recipient_id: UnsafeSlice,
  recipient_login: UnsafeSlice,
  sub_plan: UnsafeSlice,
  pub sub_plan_name: String,
  pub gift_months: i64,
  /// If the SubGift is anonymous, it means the sender
  /// (UserNoticeBase.user) will be the channel owner
  pub is_anon: bool,
}

#[derive(Debug, PartialEq)]
pub struct SubMysteryGift {
  pub base: UserNoticeBase,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct GiftPaidUpgrade {
  pub base: UserNoticeBase,
  pub promo_gift_total: i64,
  promo_name: UnsafeSlice,
  sender_login: Option<UnsafeSlice>,
  sender_name: Option<UnsafeSlice>,
  pub is_anon: bool,
}

#[derive(Debug, PartialEq)]
pub struct RewardGift {
  pub base: UserNoticeBase,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Raid {
  pub base: UserNoticeBase,
  /// Display name of raid origin channel
  pub source_display_name: String,
  /// Login of raid origin channel
  source_login: UnsafeSlice,
  pub viewer_count: i64,
}

#[derive(Debug, PartialEq)]
pub struct Unraid {
  pub base: UserNoticeBase,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Ritual {
  pub base: UserNoticeBase,
  ritual_name: UnsafeSlice,
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct BitsBadgeTier {
  pub base: UserNoticeBase,
  /// Tier of bits badge the user just earned
  threshold: UnsafeSlice,
}

#[derive(Debug, PartialEq)]
pub enum UserNotice {
  Sub(Sub),
  SubExtension(SubExtension),
  SubGift(SubGift),
  SubMysteryGift(SubMysteryGift),
  GiftPaidUpgrade(GiftPaidUpgrade),
  RewardGift(RewardGift),
  Raid(Raid),
  Unraid(Unraid),
  Ritual(Ritual),
  BitsBadgeTier(BitsBadgeTier),
}

impl UserNotice {
  fn parse(source: irc::Message) -> Result<Self> {
    let base = |source: irc::Message| -> Result<UserNoticeBase> {
      Ok(UserNoticeBase {
        channel: source
          .channel()
          .ok_or_else(|| Error::MissingParam("channel".into()))?
          .into(),
        text: source
          .params
          .as_ref()
          .map(|v| v.raw())
          .map(|v| v.trim_start().strip_prefix(':').unwrap_or(v))
          .map(|v| v.into()),
        user: TwitchUser {
          id: source.tags.require_raw("user-id")?,
          login: source.tags.require_raw("login")?,
          name: source.tags.require_ns("display-name")?,
          badge_info: source.tags.get_raw("badge-info"),
          badges: source.tags.get_raw("badges"),
        },
        color: source.tags.get_raw("color"),
        emotes: source.tags.get_raw("emotes").unwrap_or_default(),
        id: source.tags.require_raw("id")?,
        room_id: source.tags.require_raw("room-id")?,
        time: source.tags.require_date("tmi-sent-ts")?,
        system_msg: source.tags.require_ns("system-msg")?,
        raw: source,
      })
    };

    Ok(match source.tags.require_raw("msg-id")?.as_ref() {
      "sub" => UserNotice::Sub(Sub {
        cumulative_months: source.tags.require_number("msg-param-cumulative-months")?,
        should_share_streak: source.tags.get_bool("msg-param-should-share-streak").unwrap_or(false),
        streak_months: source.tags.get_number("msg-param-streak-months").unwrap_or(0),
        sub_plan: source.tags.require_raw("msg-param-sub-plan")?,
        sub_plan_name: source.tags.require_ns("msg-param-sub-plan-name")?,
        is_resub: false,
        base: base(source)?,
      }),
      "extendsub" => UserNotice::SubExtension(SubExtension {
        cumulative_months: source.tags.require_number("msg-param-cumulative-months")?,
        benefit_end_month: source.tags.require_number("msg-param-sub-benefit-end-month")?,
        sub_plan: source.tags.require_raw("msg-param-sub-plan")?,
        sub_plan_name: source.tags.require_ns("msg-param-sub-plan-name").ok(),
        base: base(source)?,
      }),
      "resub" => UserNotice::Sub(Sub {
        cumulative_months: source.tags.require_number("msg-param-cumulative-months")?,
        should_share_streak: source.tags.get_bool("msg-param-should-share-streak").unwrap_or(false),
        streak_months: source.tags.get_number("msg-param-streak-months").unwrap_or(0),
        sub_plan: source.tags.require_raw("msg-param-sub-plan")?,
        sub_plan_name: source.tags.require_ns("msg-param-sub-plan-name")?,
        is_resub: true,
        base: base(source)?,
      }),
      "subgift" => UserNotice::SubGift(SubGift {
        cumulative_months: source.tags.require_number("msg-param-months")?,
        recipient_display_name: source.tags.require_ns("msg-param-recipient-display-name")?,
        recipient_id: source.tags.require_raw("msg-param-recipient-id")?,
        recipient_login: source.tags.require_raw("msg-param-recipient-user-name")?,
        sub_plan: source.tags.require_raw("msg-param-sub-plan")?,
        sub_plan_name: source.tags.require_ns("msg-param-sub-plan-name")?,
        gift_months: source.tags.get_number("msg-param-gift-months").unwrap_or(1),
        is_anon: false,
        base: base(source)?,
      }),
      "anonsubgift" => UserNotice::SubGift(SubGift {
        cumulative_months: source.tags.require_number("msg-param-months")?,
        recipient_display_name: source.tags.require_ns("msg-param-recipient-display-name")?,
        recipient_id: source.tags.require_raw("msg-param-recipient-id")?,
        recipient_login: source.tags.require_raw("msg-param-recipient-user-name")?,
        sub_plan: source.tags.require_raw("msg-param-sub-plan")?,
        sub_plan_name: source.tags.require_ns("msg-param-sub-plan-name")?,
        gift_months: source.tags.get_number("msg-param-gift-months").unwrap_or(1),
        is_anon: true,
        base: base(source)?,
      }),
      "submysterygift" => UserNotice::SubMysteryGift(SubMysteryGift { base: base(source)? }),
      "giftpaidupgrade" => UserNotice::GiftPaidUpgrade(GiftPaidUpgrade {
        promo_gift_total: source.tags.require_number("msg-param-promo-gift-total")?,
        promo_name: source.tags.require_raw("msg-param-promo-name")?,
        sender_login: source.tags.get_raw("msg-param-sender-login"),
        sender_name: source.tags.get_raw("msg-param-sender-name"),
        is_anon: false,
        base: base(source)?,
      }),
      "anongiftpaidupgrade" => UserNotice::GiftPaidUpgrade(GiftPaidUpgrade {
        promo_gift_total: source.tags.require_number("msg-param-promo-gift-total")?,
        promo_name: source.tags.require_raw("msg-param-promo-name")?,
        sender_login: source.tags.get_raw("msg-param-sender-login"),
        sender_name: source.tags.get_raw("msg-param-sender-name"),
        is_anon: true,
        base: base(source)?,
      }),
      "rewardgift" => UserNotice::RewardGift(RewardGift { base: base(source)? }),
      "raid" => UserNotice::Raid(Raid {
        source_display_name: source.tags.require_ns("msg-param-displayName")?,
        source_login: source.tags.require_raw("msg-param-login")?,
        viewer_count: source.tags.require_number("msg-param-viewerCount")?,
        base: base(source)?,
      }),
      "unraid" => UserNotice::Unraid(Unraid { base: base(source)? }),
      "ritual" => UserNotice::Ritual(Ritual {
        ritual_name: source.tags.require_raw("msg-param-ritual-name")?,
        base: base(source)?,
      }),
      "bitsbadgetier" => UserNotice::BitsBadgeTier(BitsBadgeTier {
        threshold: source.tags.require_raw("msg-param-threshold")?,
        base: base(source)?,
      }),
      invalid => return Err(Error::InvalidTagValue("msg-id".into(), invalid.into())),
    })
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct UserState {
  channel: UnsafeSlice,
  pub display_name: String,
  badge_info: Option<UnsafeSlice>,
  #[csv]
  badges: UnsafeSlice,
  color: Option<UnsafeSlice>,
  #[csv]
  emote_sets: UnsafeSlice,
  raw: irc::Message,
}

impl UserState {
  fn parse(source: irc::Message) -> Result<Self> {
    Ok(UserState {
      channel: source
        .channel()
        .ok_or_else(|| Error::MissingParam("channel".into()))?
        .into(),
      display_name: source.tags.require_ns("display-name")?,
      badge_info: source.tags.get_raw("badge-info"),
      badges: source.tags.get_raw("badges").unwrap_or_default(),
      color: source.tags.get_raw("color"),
      emote_sets: source.tags.get_raw("emote-sets").unwrap_or_default(),
      raw: source,
    })
  }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CapabilitySubCmd {
  /// Capability.`kinds` contains a list of possible capabilities
  LS,
  /// Previous capability request was successful
  ACK,
  /// Previous capability request was not successful
  NAK,
}

impl CapabilitySubCmd {
  fn parse(which: &str) -> Option<CapabilitySubCmd> {
    match which {
      "LS" => Some(CapabilitySubCmd::LS),
      "ACK" => Some(CapabilitySubCmd::ACK),
      "NAK" => Some(CapabilitySubCmd::NAK),
      _ => None,
    }
  }
}

#[twitch_getters]
#[derive(Debug, PartialEq)]
pub struct Capability {
  pub subcmd: CapabilitySubCmd,
  which: UnsafeSlice,
  raw: irc::Message,
}

impl Capability {
  fn parse(source: irc::Message) -> Result<Self> {
    // skip the first param, which is '*'
    let params = source.params.as_ref().ok_or(Error::MalformedMessage)?;
    let params = params
      .raw()
      .trim_start()
      .strip_prefix("* ")
      .ok_or(Error::MalformedMessage)?;
    let (subcmd, which) = match params.split_once(' ') {
      Some((s, w)) => (
        CapabilitySubCmd::parse(s).ok_or(Error::MalformedMessage)?,
        w.trim_start().strip_prefix(':').ok_or(Error::MalformedMessage)?.into(),
      ),
      None => return Err(Error::MalformedMessage),
    };
    Ok(Capability {
      subcmd,
      which,
      raw: source,
    })
  }
}

#[cfg(test)]
mod tests {
  use chrono::TimeZone;
  use pretty_assertions::assert_eq;

  use super::*;

  // TODO: tests for error cases

  #[test]
  pub fn parse_ping() {
    let src = "PING :tmi.twitch.tv";

    assert_eq!(
      Message::Ping(Ping {
        arg: Some("tmi.twitch.tv".into()),
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_ping_no_arg() {
    let src = "PING";

    assert_eq!(
      Message::Ping(Ping {
        arg: None,
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_pong() {
    let src = "PONG :tmi.twitch.tv";

    assert_eq!(
      Message::Pong(Pong {
        arg: Some("tmi.twitch.tv".into()),
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_pong_no_arg() {
    let src = "PONG";

    assert_eq!(
      Message::Pong(Pong {
        arg: None,
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_join() {
    let src = ":test!test@test.tmi.twitch.tv JOIN #channel";

    assert_eq!(
      Message::Join(Join {
        channel: "channel".into(),
        nick: "test".into(),
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_part() {
    let src = ":test!test@test.tmi.twitch.tv PART #channel";

    assert_eq!(
      Message::Part(Part {
        channel: "channel".into(),
        nick: "test".into(),
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_privmsg_non_action() {
    let src = "\
        @badge-info=subscriber/5;badges=broadcaster/1,subscriber/0;\
        color=#19E6E6;display-name=randers;emotes=;flags=;id=7eb848c9-1060-4e5e-9f4c-612877982e79;\
        mod=0;room-id=40286300;subscriber=1;tmi-sent-ts=1563096499780;turbo=0;\
        user-id=40286300;user-type= :randers!randers@randers.tmi.twitch.tv PRIVMSG #randers :test\
        ";

    assert_eq!(
      Message::Privmsg(Privmsg {
        channel: "randers".into(),
        text: "test".into(),
        user: TwitchUser {
          id: "40286300".into(),
          login: "randers".into(),
          name: "randers".into(),
          badge_info: Some("subscriber/5".into()),
          badges: Some("broadcaster/1,subscriber/0".into())
        },
        is_action: false,
        bits: None,
        color: Some("#19E6E6".into()),
        emotes: "".into(),
        id: "7eb848c9-1060-4e5e-9f4c-612877982e79".into(),
        room_id: "40286300".into(),
        time: Utc.timestamp_millis(1563096499780i64),
        custom_reward_id: None,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_privmsg_action() {
    let src = "\
        @badge-info=subscriber/5;badges=broadcaster/1,subscriber/0;\
        color=#19E6E6;display-name=randers;emotes=;flags=;id=7eb848c9-1060-4e5e-9f4c-612877982e79;\
        mod=0;room-id=40286300;subscriber=1;tmi-sent-ts=1563096499780;turbo=0;\
        user-id=40286300;user-type= :randers!randers@randers.tmi.twitch.tv PRIVMSG #randers :\x01ACTION test\x01\
        ";

    assert_eq!(
      Message::Privmsg(Privmsg {
        channel: "randers".into(),
        text: "test".into(),
        user: TwitchUser {
          id: "40286300".into(),
          login: "randers".into(),
          name: "randers".into(),
          badge_info: Some("subscriber/5".into()),
          badges: Some("broadcaster/1,subscriber/0".into())
        },
        is_action: true,
        bits: None,
        color: Some("#19E6E6".into()),
        emotes: "".into(),
        id: "7eb848c9-1060-4e5e-9f4c-612877982e79".into(),
        room_id: "40286300".into(),
        time: Utc.timestamp_millis(1563096499780i64),
        custom_reward_id: None,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_whisper_non_action() {
    let src = "\
        @badges=;color=#2E8B57;display-name=pajbot;emotes=25:7-11;message-id=\
        2034;thread-id=40286300_82008718;turbo=0;user-id=82008718;user-type= \
        :pajbot!pajbot@pajbot.tmi.twitch.tv WHISPER randers :Riftey Kappa\
        ";

    assert_eq!(
      Message::Whisper(Whisper {
        recipient: "randers".into(),
        thread_id: "40286300_82008718".into(),
        user: TwitchUser {
          id: "82008718".into(),
          login: "pajbot".into(),
          name: "pajbot".into(),
          badge_info: None,
          badges: None
        },
        text: "Riftey Kappa".into(),
        is_action: false,
        color: Some("#2E8B57".into()),
        emotes: "25:7-11".into(),
        id: "2034".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_whisper_action() {
    let src = "\
        @badges=;color=#2E8B57;display-name=pajbot;emotes=25:7-11;message-id=\
        2034;thread-id=40286300_82008718;turbo=0;user-id=82008718;user-type= \
        :pajbot!pajbot@pajbot.tmi.twitch.tv WHISPER randers :\x01ACTION Riftey Kappa\x01\
        ";

    assert_eq!(
      Message::Whisper(Whisper {
        recipient: "randers".into(),
        thread_id: "40286300_82008718".into(),
        user: TwitchUser {
          id: "82008718".into(),
          login: "pajbot".into(),
          name: "pajbot".into(),
          badge_info: None,
          badges: None
        },
        text: "Riftey Kappa".into(),
        is_action: true,
        color: Some("#2E8B57".into()),
        emotes: "25:7-11".into(),
        id: "2034".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_clearchat_timeout() {
    let src = "\
        @ban-duration=600;room-id=40286300;target-user-id=70948394;tmi-sent-ts=1563051113633 \
        :tmi.twitch.tv CLEARCHAT #randers :weeb123\
        ";

    assert_eq!(
      Message::Clearchat(Clearchat {
        channel: "randers".into(),
        target: Some("weeb123".into()),
        target_id: Some("70948394".into()),
        time: Utc.timestamp_millis(1563051113633),
        duration: Some(Duration::seconds(600)),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_clearchat_permaban() {
    let src = "\
        @room-id=40286300;target-user-id=70948394;tmi-sent-ts=1563051758128 \
        :tmi.twitch.tv CLEARCHAT #randers :weeb123\
        ";

    assert_eq!(
      Message::Clearchat(Clearchat {
        channel: "randers".into(),
        target: Some("weeb123".into()),
        target_id: Some("70948394".into()),
        time: Utc.timestamp_millis(1563051758128),
        duration: None,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_clearchat() {
    let src = "\
        @room-id=40286300;tmi-sent-ts=1563051778390 \
        :tmi.twitch.tv CLEARCHAT #randers\
        ";

    assert_eq!(
      Message::Clearchat(Clearchat {
        channel: "randers".into(),
        target: None,
        target_id: None,
        time: Utc.timestamp_millis(1563051778390),
        duration: None,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_clearmsg() {
    let src = "\
        @login=supibot;room-id=;target-msg-id=25fd76d9-4731-4907-978e-a391134ebd67;\
        tmi-sent-ts=-6795364578871 :tmi.twitch.tv CLEARMSG #randers :Pong! Uptime: 6h,\
        15m; Temperature: 54.8°C; Latency to TMI: 183ms; Commands used: 795\
        ";

    assert_eq!(
      Message::Clearmsg(Clearmsg {
        channel: "randers".into(),
        login: "supibot".into(),
        /// Deleted message text
        text: "Pong! Uptime: 6h,15m; Temperature: 54.8°C; Latency to TMI: 183ms; Commands used: 795".into(),
        target_msg_id: "25fd76d9-4731-4907-978e-a391134ebd67".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_globaluserstate_0() {
    let src = "\
        @badge-info=;badges=bits-charity/1;color=#19E6E6;display-name=RANDERS;\
        emote-sets=0,42,237;user-id=40286300;user-type= \
        :tmi.twitch.tv GLOBALUSERSTATE\
        ";

    assert_eq!(
      Message::GlobalUserState(GlobalUserState {
        user_id: "40286300".into(),
        display_name: "RANDERS".into(),
        badge_info: None,
        badges: "bits-charity/1".into(),
        color: Some("#19E6E6".into()),
        emote_sets: "0,42,237".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_globaluserstate_1() {
    let src = "\
        @badge-info=;badges=;color=;display-name=receivertest3;emote-sets=0;user-id=422021310;user-type= \
        :tmi.twitch.tv GLOBALUSERSTATE\
        ";

    assert_eq!(
      Message::GlobalUserState(GlobalUserState {
        user_id: "422021310".into(),
        display_name: "receivertest3".into(),
        badge_info: None,
        badges: "".into(),
        color: None,
        emote_sets: "0".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_hosttarget_host() {
    let src = ":tmi.twitch.tv HOSTTARGET #randers :leebaxd 0";

    assert_eq!(
      Message::HostTarget(HostTarget {
        hosting_channel: "randers".into(),
        target_channel: Some("leebaxd".into()),
        viewer_count: Some(0),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_hosttarget_unhost() {
    let src = ":tmi.twitch.tv HOSTTARGET #randers :-";

    assert_eq!(
      Message::HostTarget(HostTarget {
        hosting_channel: "randers".into(),
        target_channel: None,
        viewer_count: None,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_notice_banned() {
    let src = "\
        @msg-id=msg_banned \
        :tmi.twitch.tv NOTICE #forsen :You are permanently banned from talking in forsen.\
        ";

    assert_eq!(
      Message::Notice(Notice {
        id: Some(NoticeId::MsgBanned),
        channel: Some("forsen".into()),
        message: "You are permanently banned from talking in forsen.".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_notice_bad_auth() {
    let src = ":tmi.twitch.tv NOTICE * :Improperly formatted auth";

    assert_eq!(
      Message::Notice(Notice {
        id: None,
        channel: None,
        message: "Improperly formatted auth".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_roomstate_full() {
    let src = "\
        @emote-only=0;followers-only=-1;r9k=0;rituals=0;room-id=40286300;slow=0;subs-only=0 \
        :tmi.twitch.tv ROOMSTATE #randers\
        ";

    assert_eq!(
      Message::RoomState(RoomState {
        channel: "randers".into(),
        emote_only: Some(false),
        followers_only: Some(FollowerOnlyMode::Disabled),
        r9k: Some(false),
        slow: Some(0),
        subs_only: Some(false),
        is_update: false,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_roomstate_partial() {
    let src = "@slow=10 :tmi.twitch.tv ROOMSTATE #dallas";

    assert_eq!(
      Message::RoomState(RoomState {
        channel: "dallas".into(),
        emote_only: None,
        followers_only: None,
        r9k: None,
        slow: Some(10),
        subs_only: None,
        is_update: true,
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_usernotice_resub() {
    let src = "\
        @badge-info=;badges=staff/1,broadcaster/1,turbo/1;color=#008000;\
        display-name=ronni;emotes=;id=db25007f-7a18-43eb-9379-80131e44d633;\
        login=ronni;mod=0;msg-id=resub;msg-param-cumulative-months=6;msg-param-streak-months=2;\
        msg-param-should-share-streak=1;msg-param-sub-plan=Prime;msg-param-sub-plan-name=Prime;\
        room-id=1337;subscriber=1;system-msg=ronni\\shas\\ssubscribed\\sfor\\s6\\smonths!;\
        tmi-sent-ts=1507246572675;turbo=1;user-id=1337;user-type=staff \
        :tmi.twitch.tv USERNOTICE #dallas :Great stream -- keep it up!\
        ";

    assert_eq!(
      Message::UserNotice(UserNotice::Sub(Sub {
        base: UserNoticeBase {
          channel: "dallas".into(),
          text: Some("Great stream -- keep it up!".into()),
          user: TwitchUser {
            id: "1337".into(),
            login: "ronni".into(),
            name: "ronni".into(),
            badge_info: None,
            badges: Some("staff/1,broadcaster/1,turbo/1".into())
          },
          color: Some("#008000".into()),
          emotes: "".into(),
          id: "db25007f-7a18-43eb-9379-80131e44d633".into(),
          room_id: "1337".into(),
          system_msg: "ronni has subscribed for 6 months!".into(),
          time: Utc.timestamp_millis(1507246572675),
          raw: irc::Message::parse(src).unwrap(),
        },
        cumulative_months: 6,
        should_share_streak: true,
        streak_months: 2,
        sub_plan: "Prime".into(),
        sub_plan_name: "Prime".into(),
        is_resub: true,
      })),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_usernotice_extendsub() {
    let src = "\
        @badge-info=subscriber/1;badges=staff/1,subscriber/0,premium/1;color=;display-name=olivetan;\
        emotes=;flags=;id=6031612b-bd79-4a89-a1a3-b8f3f8bc7573;login=olivetan;mod=0;msg-id=extendsub;\
        msg-param-sub-benefit-end-month=4;msg-param-sub-plan=1000;msg-param-cumulative-months=16;room-id=434858776;\
        subscriber=1;system-msg=olivetan\\sextended\\stheir\\sTier\\s1\\ssubscription\\sthrough\\sApril!;tmi-sent-ts=1565212333824;\
        user-id=433099049;user-type=staff \
        :tmi.twitch.tv USERNOTICE #pennypicklesthedog";

    assert_eq!(
      Message::UserNotice(UserNotice::SubExtension(SubExtension {
        base: UserNoticeBase {
          channel: "pennypicklesthedog".into(),
          text: None,
          user: TwitchUser {
            id: "433099049".into(),
            login: "olivetan".into(),
            name: "olivetan".into(),
            badge_info: Some("subscriber/1".into()),
            badges: Some("staff/1,subscriber/0,premium/1".into())
          },
          color: None,
          emotes: "".into(),
          id: "6031612b-bd79-4a89-a1a3-b8f3f8bc7573".into(),
          room_id: "434858776".into(),
          system_msg: "olivetan extended their Tier 1 subscription through April!".into(),
          time: Utc.timestamp_millis(1565212333824),
          raw: irc::Message::parse(src).unwrap(),
        },
        cumulative_months: 16,
        benefit_end_month: 4,
        sub_plan: "1000".into(),
        sub_plan_name: None,
      })),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_usernotice_gift() {
    let src = "\
        @badge-info=;badges=staff/1,premium/1;color=#0000FF;\
        display-name=TWW2;emotes=;id=e9176cd8-5e22-4684-ad40-ce53c2561c5e;\
        login=tww2;mod=0;msg-id=subgift;msg-param-months=1;\
        msg-param-recipient-display-name=Mr_Woodchuck;msg-param-recipient-id=89614178;\
        msg-param-recipient-user-name=mr_woodchuck;msg-param-sub-plan-name=House\\sof\\sNyoro~n;\
        msg-param-sub-plan=1000;room-id=19571752;subscriber=0;\
        system-msg=TWW2\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sMr_Woodchuck!;\
        tmi-sent-ts=1521159445153;turbo=0;user-id=13405587;user-type=staff \
        :tmi.twitch.tv USERNOTICE #forstycup\
        ";

    assert_eq!(
      Message::UserNotice(UserNotice::SubGift(SubGift {
        base: UserNoticeBase {
          channel: "forstycup".into(),
          text: None,
          user: TwitchUser {
            id: "13405587".into(),
            login: "tww2".into(),
            name: "TWW2".into(),
            badge_info: None,
            badges: Some("staff/1,premium/1".into())
          },
          color: Some("#0000FF".into()),
          emotes: "".into(),
          id: "e9176cd8-5e22-4684-ad40-ce53c2561c5e".into(),
          room_id: "19571752".into(),
          system_msg: "TWW2 gifted a Tier 1 sub to Mr_Woodchuck!".into(),
          time: Utc.timestamp_millis(1521159445153),
          raw: irc::Message::parse(src).unwrap(),
        },
        cumulative_months: 1,
        recipient_display_name: "Mr_Woodchuck".into(),
        recipient_id: "89614178".into(),
        recipient_login: "mr_woodchuck".into(),
        sub_plan: "1000".into(),
        sub_plan_name: "House of Nyoro~n".into(),
        gift_months: 1,
        is_anon: false,
      })),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_reconnect() {
    let src = ":tmi.twitch.tv RECONNECT";

    assert_eq!(
      Message::Reconnect(Reconnect {
        raw: irc::Message::parse(src).unwrap()
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_userstate() {
    let src = "\
        @badge-info=;badges=;color=#FF0000;\
        display-name=zwb3_pyramids;emote-sets=0;mod=0;subscriber=0;user-type= \
        :tmi.twitch.tv USERSTATE #randers\
        ";

    assert_eq!(
      Message::UserState(UserState {
        channel: "randers".into(),
        display_name: "zwb3_pyramids".into(),
        badge_info: None,
        badges: "".into(),
        color: Some("#FF0000".into()),
        emote_sets: "0".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_capability_ack_single() {
    let src = ":tmi.twitch.tv CAP * ACK :twitch.tv/commands";

    assert_eq!(
      Message::Capability(Capability {
        subcmd: CapabilitySubCmd::ACK,
        which: "twitch.tv/commands".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_capability_ack_multi() {
    let src = ":tmi.twitch.tv CAP * ACK :twitch.tv/commands twitch.tv/tags twitch.tv/membership";

    assert_eq!(
      Message::Capability(Capability {
        subcmd: CapabilitySubCmd::ACK,
        which: "twitch.tv/commands twitch.tv/tags twitch.tv/membership".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_capability_nak_single() {
    let src = ":tmi.twitch.tv CAP * NAK :twitch.tv/invalid";

    assert_eq!(
      Message::Capability(Capability {
        subcmd: CapabilitySubCmd::NAK,
        which: "twitch.tv/invalid".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_capability_nak_multi() {
    let src = ":tmi.twitch.tv CAP * NAK :twitch.tv/invalid0 twitch.tv/invalid1 twitch.tv/invalid2";

    assert_eq!(
      Message::Capability(Capability {
        subcmd: CapabilitySubCmd::NAK,
        which: "twitch.tv/invalid0 twitch.tv/invalid1 twitch.tv/invalid2".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_capability_ls() {
    let src = ":tmi.twitch.tv CAP * LS :twitch.tv/commands twitch.tv/tags twitch.tv/membership";

    assert_eq!(
      Message::Capability(Capability {
        subcmd: CapabilitySubCmd::LS,
        which: "twitch.tv/commands twitch.tv/tags twitch.tv/membership".into(),
        raw: irc::Message::parse(src).unwrap(),
      }),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }

  #[test]
  pub fn parse_welcome_msg() {
    let src = ":tmi.twitch.tv 001 justinfan12345 :Welcome, GLHF!";

    assert_eq!(
      Message::Unknown(irc::Message::parse(src).unwrap()),
      Message::try_from(irc::Message::parse(src).unwrap()).unwrap()
    )
  }
}

/* list of tag names:
"ban-duration"
"login"
"target-msg-id"
"badge-info"
"badges"
"bits"
"color"
"display-name"
"emote-sets"
"emotes"
"extendsub"
"id"
"msg-id"
"system-msg"
"room-id"
"user-id"
"emote-only"
"followers-only"
"r9k"
"slow"
"subs-only"
"tmi-sent-ts"
"msg-param-cumulative-months"
"msg-param-displayName"
"msg-param-login"
"msg-param-months"
"msg-param-promo-gift-total"
"msg-param-promo-name"
"msg-param-recipient-display-name"
"msg-param-recipient-id"
"msg-param-recipient-user-name"
"msg-param-sender-login"
"msg-param-sender-name"
"msg-param-should-share-streak"
"msg-param-streak-months"
"msg-param-sub-plan"
"msg-param-sub-plan-name"
"msg-param-viewerCount"
"msg-param-ritual-name"
"msg-param-threshold"
"msg-param-gift-months"
"thread-id"
"message-id"
"custom-reward-id"
*/
