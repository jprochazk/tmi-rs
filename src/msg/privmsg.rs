//! Represents a basic Twitch chat message sent by some user to a specific channel.

use super::{
  is_not_empty, parse_badges, parse_message_text, parse_timestamp, Badge, MessageParseError, User,
};
use crate::common::{maybe_unescape, ChannelRef, MaybeOwned};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};
use std::borrow::Cow;

/// Represents a basic Twitch chat message sent by some user to a specific channel.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Privmsg<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: MaybeOwned<'src, ChannelRef>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  channel_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  message_id: Cow<'src, str>,

  sender: User<'src>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  reply_to: Option<Reply<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,

  is_action: bool,

  #[cfg_attr(feature = "serde", serde(borrow))]
  badges: Vec<Badge<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  color: Option<Cow<'src, str>>,

  bits: Option<u64>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  emotes: Cow<'src, str>,

  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for Privmsg<'src> as self {
    /// Channel in which this message was sent.
    channel -> &ChannelRef = self.channel.as_ref(),

    /// ID of the channel in which this message was sent.
    channel_id -> &str = self.channel_id.as_ref(),

    /// Unique ID of the message.
    message_id -> &str = self.message_id.as_ref(),

    /// Basic info about the user who sent this message.
    sender -> &User<'src> = &self.sender,

    /// Info about the parent message this message is a reply.
    reply_to -> Option<&Reply<'src>> = self.reply_to.as_ref(),

    /// Text content of the message.
    ///
    /// This strips the action prefix/suffix bytes if the message was sent with `/me`.
    text -> &str = self.text.as_ref(),

    /// Whether the message was sent with `/me`.
    is_action -> bool,

    /// Iterator over the channel badges enabled by the user in the [channel][`Privmsg::channel`].
    badges -> impl Iterator<Item = &Badge<'src>> = self.badges.iter(),

    /// Number of channel badges enabled by the user in the [channel][`Privmsg::channel`].
    num_badges -> usize = self.badges.len(),

    /// The user's selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str> = self.color.as_deref(),

    /// The number of bits gifted with this message.
    bits -> Option<u64>,

    /// The emote raw emote ranges present in this message.
    ///
    /// ⚠ Note: This is _hopelessly broken_ and should **never be used for any purpose whatsoever**,
    /// you should instead parse the emotes yourself out of the message according to the available emote sets.
    /// If for some reason you need it, here you go.
    raw_emotes -> &str = self.emotes.as_ref(),

    /// The time at which the message was sent.
    timestamp -> DateTime<Utc>,
  }
}

/// Information about the reply parent message.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Reply<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  thread_message_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  thread_user_login: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  message_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  sender: User<'src>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,
}

generate_getters! {
  <'src> for Reply<'src> as self {
    /// Reply thread parent message ID
    thread_message_id -> &str = self.thread_message_id.as_ref(),

    /// Reply thread parent user login
    thread_user_login -> &str = self.thread_user_login.as_ref(),

    /// Reply parent message ID
    message_id -> &str = self.message_id.as_ref(),

    /// Reply parent sender
    sender -> User<'src>,

    /// Reply parent text
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    text -> Cow<'src, str> = maybe_unescape(self.text.clone()),
  }
}

impl<'src> Privmsg<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Privmsg {
      return None;
    }

    let reply_to = message.tag(Tag::ReplyParentMsgId).and_then(|message_id| {
      Some(Reply {
        thread_message_id: message.tag(Tag::ReplyThreadParentMsgId)?.into(),
        thread_user_login: message.tag(Tag::ReplyThreadParentUserLogin)?.into(),
        message_id: message_id.into(),
        sender: User {
          id: message.tag(Tag::ReplyParentUserId)?.into(),
          login: message.tag(Tag::ReplyParentUserLogin)?.into(),
          name: message.tag(Tag::ReplyParentDisplayName)?.into(),
        },
        text: message.tag(Tag::ReplyParentMsgBody)?.into(),
      })
    });

    let (text, is_action) = parse_message_text(message.text()?);
    Some(Privmsg {
      channel: MaybeOwned::Ref(message.channel()?),
      channel_id: message.tag(Tag::RoomId)?.into(),
      message_id: message.tag(Tag::Id)?.into(),
      sender: User {
        id: message.tag(Tag::UserId)?.into(),
        login: message
          .prefix()
          .and_then(|prefix| prefix.nick)
          .map(Cow::Borrowed)?,
        name: message.tag(Tag::DisplayName)?.into(),
      },
      reply_to,
      text: text.into(),
      is_action,
      badges: message
        .tag(Tag::Badges)
        .zip(message.tag(Tag::BadgeInfo))
        .map(|(badges, badge_info)| parse_badges(badges, badge_info))
        .unwrap_or_default(),
      color: message
        .tag(Tag::Color)
        .filter(is_not_empty)
        .map(Cow::Borrowed),
      bits: message.tag(Tag::Bits).and_then(|bits| bits.parse().ok()),
      emotes: message.tag(Tag::Emotes).unwrap_or_default().into(),
      timestamp: message.tag(Tag::TmiSentTs).and_then(parse_timestamp)?,
    })
  }
}

impl<'src> super::FromIrc<'src> for Privmsg<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<Privmsg<'src>> for super::Message<'src> {
  fn from(msg: Privmsg<'src>) -> Self {
    super::Message::Privmsg(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_privmsg_basic_example() {
    assert_irc_snapshot!(Privmsg, "@badge-info=;badges=;color=#0000FF;display-name=JuN1oRRRR;emotes=;flags=;id=e9d998c3-36f1-430f-89ec-6b887c28af36;mod=0;room-id=11148817;subscriber=0;tmi-sent-ts=1594545155039;turbo=0;user-id=29803735;user-type= :jun1orrrr!jun1orrrr@jun1orrrr.tmi.twitch.tv PRIVMSG #pajlada :dank cam");
  }

  #[test]
  fn parse_privmsg_action_and_badges() {
    assert_irc_snapshot!(Privmsg, "@badge-info=subscriber/22;badges=moderator/1,subscriber/12;color=#19E6E6;display-name=randers;emotes=;flags=;id=d831d848-b7c7-4559-ae3a-2cb88f4dbfed;mod=1;room-id=11148817;subscriber=1;tmi-sent-ts=1594555275886;turbo=0;user-id=40286300;user-type=mod :randers!randers@randers.tmi.twitch.tv PRIVMSG #pajlada :ACTION -tags");
  }

  #[test]
  fn parse_privmsg_reply_parent_included() {
    assert_irc_snapshot!(Privmsg, "@badge-info=;badges=;client-nonce=cd56193132f934ac71b4d5ac488d4bd6;color=;display-name=LeftSwing;emotes=;first-msg=0;flags=;id=5b4f63a9-776f-4fce-bf3c-d9707f52e32d;mod=0;reply-parent-display-name=Retoon;reply-parent-msg-body=hello;reply-parent-msg-id=6b13e51b-7ecb-43b5-ba5b-2bb5288df696;reply-parent-user-id=37940952;reply-parent-user-login=retoon;reply-thread-parent-msg-id=6b13e51b-7ecb-43b5-ba5b-2bb5288df696;reply-thread-parent-user-login=retoon;returning-chatter=0;room-id=37940952;subscriber=0;tmi-sent-ts=1673925983585;turbo=0;user-id=133651738;user-type= :leftswing!leftswing@leftswing.tmi.twitch.tv PRIVMSG #retoon :@Retoon yes");
  }

  #[test]
  fn parse_privmsg_display_name_with_trailing_space() {
    assert_irc_snapshot!(Privmsg, "@rm-received-ts=1594554085918;historical=1;badge-info=;badges=;client-nonce=815810609edecdf4537bd9586994182b;color=;display-name=CarvedTaleare\\s;emotes=;flags=;id=c9b941d9-a0ab-4534-9903-971768fcdf10;mod=0;room-id=22484632;subscriber=0;tmi-sent-ts=1594554085753;turbo=0;user-id=467684514;user-type= :carvedtaleare!carvedtaleare@carvedtaleare.tmi.twitch.tv PRIVMSG #forsen :NaM");
  }

  #[test]
  fn parse_privmsg_korean_display_name() {
    assert_irc_snapshot!(Privmsg, "@badge-info=subscriber/35;badges=moderator/1,subscriber/3024;color=#FF0000;display-name=테스트계정420;emotes=;flags=;id=bdfa278e-11c4-484f-9491-0a61b16fab60;mod=1;room-id=11148817;subscriber=1;tmi-sent-ts=1593953876927;turbo=0;user-id=117166826;user-type=mod :testaccount_420!testaccount_420@testaccount_420.tmi.twitch.tv PRIVMSG #pajlada :@asd");
  }

  #[test]
  fn parse_privmsg_display_name_with_middle_space() {
    assert_irc_snapshot!(Privmsg, "@badge-info=;badges=;color=;display-name=Riot\\sGames;emotes=;flags=;id=bdfa278e-11c4-484f-9491-0a61b16fab60;mod=1;room-id=36029255;subscriber=0;tmi-sent-ts=1593953876927;turbo=0;user-id=36029255;user-type= :riotgames!riotgames@riotgames.tmi.twitch.tv PRIVMSG #riotgames :test fake message");
  }

  #[test]
  fn parse_privmsg_emotes_1() {
    assert_irc_snapshot!(
      Privmsg,
      "@badge-info=;badges=moderator/1;client-nonce=fc4ebe0889105c8404a9be81cf9a9ad4;color=#FF0000;display-name=boring_nick;emotes=555555591:51-52/25:0-4,12-16,18-22/1902:6-10,29-33,35-39/1:45-46,48-49;first-msg=0;flags=;id=3d9540a0-04b6-4bea-baf9-9165b14160be;mod=1;returning-chatter=0;room-id=55203741;subscriber=0;tmi-sent-ts=1696093084212;turbo=0;user-id=111024753;user-type=mod :boring_nick!boring_nick@boring_nick.tmi.twitch.tv PRIVMSG #moscowwbish :Kappa Keepo Kappa Kappa test Keepo Keepo 123 :) :) :P"
    );
  }

  #[test]
  fn parse_privmsg_message_with_bits() {
    assert_irc_snapshot!(Privmsg, "@badge-info=;badges=bits/100;bits=1;color=#004B49;display-name=TETYYS;emotes=;flags=;id=d7f03a35-f339-41ca-b4d4-7c0721438570;mod=0;room-id=11148817;subscriber=0;tmi-sent-ts=1594571566672;turbo=0;user-id=36175310;user-type= :tetyys!tetyys@tetyys.tmi.twitch.tv PRIVMSG #pajlada :trihard1");
  }

  #[test]
  fn parse_privmsg_emote_non_numeric_id() {
    assert_irc_snapshot!(Privmsg, "@badge-info=;badges=;client-nonce=245b864d508a69a685e25104204bd31b;color=#FF144A;display-name=AvianArtworks;emote-only=1;emotes=300196486_TK:0-7;flags=;id=21194e0d-f0fa-4a8f-a14f-3cbe89366ad9;mod=0;room-id=11148817;subscriber=0;tmi-sent-ts=1594552113129;turbo=0;user-id=39565465;user-type= :avianartworks!avianartworks@avianartworks.tmi.twitch.tv PRIVMSG #pajlada :pajaM_TK");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_basic_example() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=;badges=;color=#0000FF;display-name=JuN1oRRRR;emotes=;flags=;id=e9d998c3-36f1-430f-89ec-6b887c28af36;mod=0;room-id=11148817;subscriber=0;tmi-sent-ts=1594545155039;turbo=0;user-id=29803735;user-type= :jun1orrrr!jun1orrrr@jun1orrrr.tmi.twitch.tv PRIVMSG #pajlada :dank cam");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_action_and_badges() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=subscriber/22;badges=moderator/1,subscriber/12;color=#19E6E6;display-name=randers;emotes=;flags=;id=d831d848-b7c7-4559-ae3a-2cb88f4dbfed;mod=1;room-id=11148817;subscriber=1;tmi-sent-ts=1594555275886;turbo=0;user-id=40286300;user-type=mod :randers!randers@randers.tmi.twitch.tv PRIVMSG #pajlada :ACTION -tags");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_reply_parent_included() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=;badges=;client-nonce=cd56193132f934ac71b4d5ac488d4bd6;color=;display-name=LeftSwing;emotes=;first-msg=0;flags=;id=5b4f63a9-776f-4fce-bf3c-d9707f52e32d;mod=0;reply-parent-display-name=Retoon;reply-parent-msg-body=hello;reply-parent-msg-id=6b13e51b-7ecb-43b5-ba5b-2bb5288df696;reply-parent-user-id=37940952;reply-parent-user-login=retoon;reply-thread-parent-msg-id=6b13e51b-7ecb-43b5-ba5b-2bb5288df696;reply-thread-parent-user-login=retoon;returning-chatter=0;room-id=37940952;subscriber=0;tmi-sent-ts=1673925983585;turbo=0;user-id=133651738;user-type= :leftswing!leftswing@leftswing.tmi.twitch.tv PRIVMSG #retoon :@Retoon yes");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_display_name_with_trailing_space() {
    assert_irc_roundtrip!(Privmsg, "@rm-received-ts=1594554085918;historical=1;badge-info=;badges=;client-nonce=815810609edecdf4537bd9586994182b;color=;display-name=CarvedTaleare\\s;emotes=;flags=;id=c9b941d9-a0ab-4534-9903-971768fcdf10;mod=0;room-id=22484632;subscriber=0;tmi-sent-ts=1594554085753;turbo=0;user-id=467684514;user-type= :carvedtaleare!carvedtaleare@carvedtaleare.tmi.twitch.tv PRIVMSG #forsen :NaM");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_korean_display_name() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=subscriber/35;badges=moderator/1,subscriber/3024;color=#FF0000;display-name=테스트계정420;emotes=;flags=;id=bdfa278e-11c4-484f-9491-0a61b16fab60;mod=1;room-id=11148817;subscriber=1;tmi-sent-ts=1593953876927;turbo=0;user-id=117166826;user-type=mod :testaccount_420!testaccount_420@testaccount_420.tmi.twitch.tv PRIVMSG #pajlada :@asd");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_display_name_with_middle_space() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=;badges=;color=;display-name=Riot\\sGames;emotes=;flags=;id=bdfa278e-11c4-484f-9491-0a61b16fab60;mod=1;room-id=36029255;subscriber=0;tmi-sent-ts=1593953876927;turbo=0;user-id=36029255;user-type= :riotgames!riotgames@riotgames.tmi.twitch.tv PRIVMSG #riotgames :test fake message");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_emotes_1() {
    assert_irc_roundtrip!(
      Privmsg,
      "@badge-info=;badges=moderator/1;client-nonce=fc4ebe0889105c8404a9be81cf9a9ad4;color=#FF0000;display-name=boring_nick;emotes=555555591:51-52/25:0-4,12-16,18-22/1902:6-10,29-33,35-39/1:45-46,48-49;first-msg=0;flags=;id=3d9540a0-04b6-4bea-baf9-9165b14160be;mod=1;returning-chatter=0;room-id=55203741;subscriber=0;tmi-sent-ts=1696093084212;turbo=0;user-id=111024753;user-type=mod :boring_nick!boring_nick@boring_nick.tmi.twitch.tv PRIVMSG #moscowwbish :Kappa Keepo Kappa Kappa test Keepo Keepo 123 :) :) :P"
    );
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_message_with_bits() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=;badges=bits/100;bits=1;color=#004B49;display-name=TETYYS;emotes=;flags=;id=d7f03a35-f339-41ca-b4d4-7c0721438570;mod=0;room-id=11148817;subscriber=0;tmi-sent-ts=1594571566672;turbo=0;user-id=36175310;user-type= :tetyys!tetyys@tetyys.tmi.twitch.tv PRIVMSG #pajlada :trihard1");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_emote_non_numeric_id() {
    assert_irc_roundtrip!(Privmsg, "@badge-info=;badges=;client-nonce=245b864d508a69a685e25104204bd31b;color=#FF144A;display-name=AvianArtworks;emote-only=1;emotes=300196486_TK:0-7;flags=;id=21194e0d-f0fa-4a8f-a14f-3cbe89366ad9;mod=0;room-id=11148817;subscriber=0;tmi-sent-ts=1594552113129;turbo=0;user-id=39565465;user-type= :avianartworks!avianartworks@avianartworks.tmi.twitch.tv PRIVMSG #pajlada :pajaM_TK");
  }
}
