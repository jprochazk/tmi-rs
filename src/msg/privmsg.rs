//! Represents a basic Twitch chat message sent by some user to a specific channel.

use super::parse_bool;
use super::{
  is_not_empty, maybe_clone, maybe_unescape, parse_badges, parse_message_text, parse_timestamp,
  Badge, MessageParseError, User,
};
use crate::irc::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};
use std::borrow::Cow;
use std::str::FromStr;

/// Represents a basic Twitch chat message sent by some user to a specific channel.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Privmsg<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  channel_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  msg_id: Option<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  id: Cow<'src, str>,

  sender: User<'src>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  reply_to: Option<Reply<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  pinned_chat: Option<PinnedChat<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,

  is_action: bool,

  #[cfg_attr(feature = "serde", serde(borrow))]
  badges: Vec<Badge<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  color: Option<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  custom_reward_id: Option<Cow<'src, str>>,

  bits: Option<u64>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  emotes: Cow<'src, str>,

  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for Privmsg<'src> as self {
    /// Unique ID of the message.
    id -> &str = self.id.as_ref(),

    /// Channel in which this message was sent.
    channel -> &str = self.channel.as_ref(),

    /// ID of the channel in which this message was sent.
    channel_id -> &str = self.channel_id.as_ref(),

    /// The `msg-id` tag, representing the type of message.
    ///
    /// This is sent for special kinds of messages, such as newly added power-ups.
    /// Example:
    /// - `msg-id=gigantified-emote-message` = last emote in the message should be
    ///   displayed in a large size on a new line, separate from the rest of the message.
    msg_id -> Option<&str> = self.msg_id.as_deref(),

    /// Basic info about the user who sent this message.
    sender -> &User<'src> = &self.sender,

    /// Info about the parent message this message is a reply.
    reply_to -> Option<&Reply<'src>> = self.reply_to.as_ref(),

    /// Info about the pinned message this message is pinned to.
    pinned_chat -> Option<&PinnedChat<'src>> = self.pinned_chat.as_ref(),

    /// Text content of the message.
    ///
    /// This strips the action prefix/suffix bytes if the message was sent with `/me`.
    text -> &str = self.text.as_ref(),

    /// Whether the message was sent with `/me`.
    is_action -> bool,

    /// Iterator over the channel badges enabled by the user in the [channel][`Privmsg::channel`].
    badges -> impl DoubleEndedIterator<Item = &Badge<'src>> + ExactSizeIterator
      = self.badges.iter(),

    /// Number of channel badges enabled by the user in the [channel][`Privmsg::channel`].
    num_badges -> usize = self.badges.len(),

    /// The user's selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str> = self.color.as_deref(),

    /// ID of the custom reward/redeem
    ///
    /// Note: This is only provided for redeems with a message.
    custom_reward_id -> Option<&str> = self.custom_reward_id.as_deref(),

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
  thread_parent_message_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  thread_parent_user_login: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  message_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  sender: User<'src>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Cow<'src, str>,
}

generate_getters! {
  <'src> for Reply<'src> as self {
    /// Root message ID of the thread the user replied to.
    ///
    /// This never changes for a given thread, so it can be used to identify the thread.
    thread_parent_message_id -> &str = self.thread_parent_message_id.as_ref(),

    /// Login of the user who posted the root message in the thread the user replied to.
    ///
    /// Twitch does not provide the display name or the user ID for this user, only
    /// their login name.
    thread_parent_user_login -> &str = self.thread_parent_user_login.as_ref(),

    /// ID of the message the user replied to directly.
    ///
    /// This is different from `thread_parent_message_id` as it identifies the specific message
    /// the user replied to, not the thread.
    message_id -> &str = self.message_id.as_ref(),

    /// Sender of the message the user replied to directly.
    sender -> User<'src>,

    /// Text of the message the user replied to directly.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    text -> Cow<'src, str> = maybe_unescape(self.text.clone()),
  }
}

/// Information about the pinned message.
///
/// If someone sent a Hype Chat, `pinned-chat-paid-*` tags would be set to reflect that.
///
/// Any currency that is used will carry information in tags that will indicate
/// the ISO 4217 currency code, and the currency’s exponent.
///
/// In the case of the United States dollar, $2 USD will be represented as 200 in
/// `pinned-chat-paid-amount` with the pinned-chat-paid-exponent of 2.
/// This indicates the decimal place is 2 decimals from the right.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PinnedChat<'src> {
  paid_amount: i64,

  #[cfg_attr(feature = "serde", serde(borrow))]
  paid_currency: Cow<'src, str>,

  paid_exponent: i64,

  paid_level: PinnedChatLevel,

  is_system_message: bool,
}

/// The level of the Hype Chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum PinnedChatLevel {
  ONE,
  TWO,
  THREE,
  FOUR,
  FIVE,
  SIX,
  SEVEN,
  EIGHT,
  NINE,
  TEN,
}

impl PinnedChatLevel {
  pub const MIN: PinnedChatLevel = PinnedChatLevel::ONE;
  pub const MAX: PinnedChatLevel = PinnedChatLevel::TEN;

  pub fn as_str(&self) -> &'static str {
    match self {
      PinnedChatLevel::ONE => "ONE",
      PinnedChatLevel::TWO => "TWO",
      PinnedChatLevel::THREE => "THREE",
      PinnedChatLevel::FOUR => "FOUR",
      PinnedChatLevel::FIVE => "FIVE",
      PinnedChatLevel::SIX => "SIX",
      PinnedChatLevel::SEVEN => "SEVEN",
      PinnedChatLevel::EIGHT => "EIGHT",
      PinnedChatLevel::NINE => "NINE",
      PinnedChatLevel::TEN => "TEN",
    }
  }
}

impl From<PinnedChatLevel> for u8 {
  fn from(level: PinnedChatLevel) -> u8 {
    level as u8
  }
}

impl TryFrom<u8> for PinnedChatLevel {
  type Error = ();

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    if value < PinnedChatLevel::MIN as u8 || value > PinnedChatLevel::MAX as u8 {
      return Err(());
    }

    Ok(unsafe { std::mem::transmute::<u8, PinnedChatLevel>(value) })
  }
}

impl FromStr for PinnedChatLevel {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "ONE" => Ok(PinnedChatLevel::ONE),
      "TWO" => Ok(PinnedChatLevel::TWO),
      "THREE" => Ok(PinnedChatLevel::THREE),
      "FOUR" => Ok(PinnedChatLevel::FOUR),
      "FIVE" => Ok(PinnedChatLevel::FIVE),
      "SIX" => Ok(PinnedChatLevel::SIX),
      "SEVEN" => Ok(PinnedChatLevel::SEVEN),
      "EIGHT" => Ok(PinnedChatLevel::EIGHT),
      "NINE" => Ok(PinnedChatLevel::NINE),
      "TEN" => Ok(PinnedChatLevel::TEN),
      _ => Err(()),
    }
  }
}

generate_getters! {
  <'src> for PinnedChat<'src> as self {
    /// The value of the Hype Chat sent by the user.
    paid_amount -> i64 = self.paid_amount,

    /// The ISO 4217 alphabetic currency code the user has sent the Hype Chat in.
    paid_currency -> &str = self.paid_currency.as_ref(),

    /// Indicates how many decimal points this currency represents partial amounts in.
    ///
    /// For example, if the exponent is `2`, then the amount will be divided by 100 to get the actual value:
    ///   `$2` -> amount = `200`, exponent = `2`, currency = "USD"
    paid_exponent -> i64 = self.paid_exponent,

    /// The level of the Hype Chat, in English.
    ///
    /// Possible values are capitalized words from `ONE` to `TEN`: ONE TWO THREE FOUR FIVE SIX SEVEN EIGHT NINE TEN
    paid_level -> PinnedChatLevel = self.paid_level,

    /// A Boolean value that determines if the message sent with the Hype Chat was filled in by the system.
    ///
    /// If `true` (1), the user entered no message and the body message was automatically filled in by the system.
    /// If `false` (0), the user provided their own message to send with the Hype Chat.
    is_system_message -> bool = self.is_system_message,
  }
}

/*

  channel: message.channel()?.into(),
  channel_id: message.tag(Tag::RoomId)?.into(),
  msg_id: message.tag(Tag::MsgId).map(Cow::Borrowed),
  id: message.tag(Tag::Id)?.into(),
  sender: User {
    id: message.tag(Tag::UserId)?.into(),
    login: message
      .prefix()
      .and_then(|prefix| prefix.nick)
      .map(Cow::Borrowed)?,
    name: message.tag(Tag::DisplayName)?.into(),
  },
  reply_to,
  pinned_chat,
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
  custom_reward_id: message
    .tag(Tag::CustomRewardId)
    .filter(is_not_empty)
    .map(Cow::Borrowed),
  bits: message.tag(Tag::Bits).and_then(|bits| bits.parse().ok()),
  emotes: message.tag(Tag::Emotes).unwrap_or_default().into(),
  timestamp: message.tag(Tag::TmiSentTs).and_then(parse_timestamp)?,
*/

impl<'src> Privmsg<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::Privmsg {
      return None;
    }

    let (text, is_action) = parse_message_text(message.text()?);
    let channel = message.channel()?.into();
    let channel_id = message.tag(Tag::RoomId)?.into();
    let msg_id = message.tag(Tag::MsgId).map(Cow::Borrowed);
    let id = message.tag(Tag::Id)?.into();
    let sender = User {
      id: message.tag(Tag::UserId)?.into(),
      login: message
        .prefix()
        .and_then(|prefix| prefix.nick)
        .map(Cow::Borrowed)?,
      name: message.tag(Tag::DisplayName)?.into(),
    };
    let reply_to = message.tag(Tag::ReplyParentMsgId).and_then(|message_id| {
      Some(Reply {
        thread_parent_message_id: message.tag(Tag::ReplyThreadParentMsgId)?.into(),
        thread_parent_user_login: message.tag(Tag::ReplyThreadParentUserLogin)?.into(),
        message_id: message_id.into(),
        sender: User {
          id: message.tag(Tag::ReplyParentUserId)?.into(),
          login: message.tag(Tag::ReplyParentUserLogin)?.into(),
          name: message.tag(Tag::ReplyParentDisplayName)?.into(),
        },
        text: message.tag(Tag::ReplyParentMsgBody)?.into(),
      })
    });
    let pinned_chat = message.tag(Tag::PinnedChatPaidAmount).and_then(|amount| {
      let paid_amount = amount.parse().ok()?;
      let paid_currency = message.tag(Tag::PinnedChatPaidCurrency)?.into();
      let paid_exponent = message.tag(Tag::PinnedChatPaidExponent)?.parse().ok()?;
      let paid_level = message.tag(Tag::PinnedChatPaidLevel)?.parse().ok()?;
      let is_system_message = parse_bool(message.tag(Tag::PinnedChatPaidIsSystemMessage)?);
      Some(PinnedChat {
        paid_amount,
        paid_currency,
        paid_exponent,
        paid_level,
        is_system_message,
      })
    });
    let text = text.into();
    let badges = message
      .tag(Tag::Badges)
      .zip(message.tag(Tag::BadgeInfo))
      .map(|(badges, badge_info)| parse_badges(badges, badge_info))
      .unwrap_or_default();
    let color = message
      .tag(Tag::Color)
      .filter(is_not_empty)
      .map(Cow::Borrowed);
    let custom_reward_id = message
      .tag(Tag::CustomRewardId)
      .filter(is_not_empty)
      .map(Cow::Borrowed);
    let bits = message.tag(Tag::Bits).and_then(|bits| bits.parse().ok());
    let emotes = message.tag(Tag::Emotes).unwrap_or_default().into();
    let timestamp = parse_timestamp(message.tag(Tag::TmiSentTs)?)?;

    Some(Privmsg {
      channel,
      channel_id,
      msg_id,
      id,
      sender,
      reply_to,
      pinned_chat,
      text,
      is_action,
      badges,
      color,
      custom_reward_id,
      bits,
      emotes,
      timestamp,
    })
  }

  /// Clone data to give the value a `'static` lifetime.
  pub fn into_owned(self) -> Privmsg<'static> {
    Privmsg {
      channel: maybe_clone(self.channel),
      channel_id: maybe_clone(self.channel_id),
      msg_id: self.msg_id.map(maybe_clone),
      id: maybe_clone(self.id),
      sender: self.sender.into_owned(),
      reply_to: self.reply_to.map(Reply::into_owned),
      pinned_chat: self.pinned_chat.map(PinnedChat::into_owned),
      text: maybe_clone(self.text),
      is_action: self.is_action,
      badges: self.badges.into_iter().map(Badge::into_owned).collect(),
      color: self.color.map(maybe_clone),
      custom_reward_id: self.custom_reward_id.map(maybe_clone),
      bits: self.bits,
      emotes: maybe_clone(self.emotes),
      timestamp: self.timestamp,
    }
  }
}

impl<'src> Reply<'src> {
  /// Clone data to give the value a `'static` lifetime.
  pub fn into_owned(self) -> Reply<'static> {
    Reply {
      thread_parent_message_id: maybe_clone(self.thread_parent_message_id),
      thread_parent_user_login: maybe_clone(self.thread_parent_user_login),
      message_id: maybe_clone(self.message_id),
      sender: self.sender.into_owned(),
      text: maybe_clone(self.text),
    }
  }
}

impl<'src> PinnedChat<'src> {
  /// Clone data to give the value a `'static` lifetime.
  pub fn into_owned(self) -> PinnedChat<'static> {
    PinnedChat {
      paid_amount: self.paid_amount,
      paid_currency: maybe_clone(self.paid_currency),
      paid_exponent: self.paid_exponent,
      paid_level: self.paid_level,
      is_system_message: self.is_system_message,
    }
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

  #[test]
  fn parse_privmsg_custom_reward_id() {
    assert_irc_snapshot!(Privmsg, "@badge-info=subscriber/1;badges=broadcaster/1,subscriber/0;color=#8A2BE2;custom-reward-id=be22f712-8fd9-426a-90df-c13eae6cc6dc;display-name=vesdeg;emotes=;first-msg=0;flags=;id=79828352-d979-4e49-bd5e-15c487d275e2;mod=0;returning-chatter=0;room-id=164774298;subscriber=1;tmi-sent-ts=1709298826724;turbo=0;user-id=164774298;user-type= :vesdeg!vesdeg@vesdeg.tmi.twitch.tv PRIVMSG #vesdeg :#00FF00");
  }

  #[test]
  fn parse_privmsg_pinned_chat() {
    // @badge-info=;badges=glhf-pledge/1;color=;emotes=;first-msg=0;flags=;id=f6fb34f8-562f-4b4d-b628-32113d0ef4b0;mod=0;pinned-chat-paid-amount=200;pinned-chat-paid-canonical-amount=200;pinned-chat-paid-currency=USD;pinned-chat-paid-exponent=2;pinned-chat-paid-is-system-message=0;pinned-chat-paid-level=ONE;returning-chatter=0;room-id=12345678;subscriber=0;tmi-sent-ts=1687471984306;turbo=0;user-id=12345678;user-type=
    assert_irc_snapshot!(Privmsg, "@badge-info=;badges=glhf-pledge/1;color=;display-name=pajlada;emotes=;first-msg=0;flags=;id=f6fb34f8-562f-4b4d-b628-32113d0ef4b0;mod=0;pinned-chat-paid-amount=200;pinned-chat-paid-canonical-amount=200;pinned-chat-paid-currency=USD;pinned-chat-paid-exponent=2;pinned-chat-paid-is-system-message=0;pinned-chat-paid-level=ONE;returning-chatter=0;room-id=12345678;subscriber=0;tmi-sent-ts=1687471984306;turbo=0;user-id=12345678;user-type= :pajlada!pajlada@pajlada.tmi.twitch.tv PRIVMSG #channel :This is a pinned message");
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

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_privmsg_pinned_chat() {
    // @badge-info=;badges=glhf-pledge/1;color=;emotes=;first-msg=0;flags=;id=f6fb34f8-562f-4b4d-b628-32113d0ef4b0;mod=0;pinned-chat-paid-amount=200;pinned-chat-paid-canonical-amount=200;pinned-chat-paid-currency=USD;pinned-chat-paid-exponent=2;pinned-chat-paid-is-system-message=0;pinned-chat-paid-level=ONE;returning-chatter=0;room-id=12345678;subscriber=0;tmi-sent-ts=1687471984306;turbo=0;user-id=12345678;user-type=
    assert_irc_roundtrip!(Privmsg, "@badge-info=;badges=glhf-pledge/1;color=;display-name=pajlada;emotes=;first-msg=0;flags=;id=f6fb34f8-562f-4b4d-b628-32113d0ef4b0;mod=0;pinned-chat-paid-amount=200;pinned-chat-paid-canonical-amount=200;pinned-chat-paid-currency=USD;pinned-chat-paid-exponent=2;pinned-chat-paid-is-system-message=0;pinned-chat-paid-level=ONE;returning-chatter=0;room-id=12345678;subscriber=0;tmi-sent-ts=1687471984306;turbo=0;user-id=12345678;user-type= :pajlada!pajlada@pajlada.tmi.twitch.tv PRIVMSG #channel :This is a pinned message");
  }

  #[test]
  fn regression_invalid_prefix_span_overread() {
    Privmsg::parse(IrcMessageRef::parse("@badge-info=;badges=moments/1;color=;display-name=kovacicdusko2001;emotes=;first-msg=0;flags=;id=97798b78-b5c7-4a0a-bcd4-e9ec12de926a;mod=0;returning-chatter=0;room-id=71092938;subscriber=0;tmi-sent-ts=1663858872621;turbo=0;user-id=251524724;user-type= :kovacicdusko2001!kovacicdusko2001@kovacicdusko2001.tmi.twitch.tv PRIVMSG #xqc :!play").unwrap()).unwrap();
  }
}
