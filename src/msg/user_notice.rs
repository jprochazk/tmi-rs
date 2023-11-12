//! A user notice is sent when some [`Event`] occurs.

use super::{is_not_empty, parse_badges, parse_timestamp, Badge, MessageParseError, User};
use crate::common::{maybe_unescape, ChannelRef, MaybeOwned};
use crate::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};
use std::borrow::Cow;

// TODO: rewardgift, primepaidupgrade, extendsub, standardpayforward, communitypayforward

/// A user notice is sent when some [`Event`] occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UserNotice<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  channel: MaybeOwned<'src, ChannelRef>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  channel_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  sender: Option<User<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  text: Option<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  system_message: Option<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  event: Event<'src>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  event_id: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  badges: Vec<Badge<'src>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  emotes: Cow<'src, str>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  color: Option<Cow<'src, str>>,

  #[cfg_attr(feature = "serde", serde(borrow))]
  message_id: Cow<'src, str>,

  timestamp: DateTime<Utc>,
}

generate_getters! {
  <'src> for UserNotice<'src> as self {
    /// Name of the channel which received this user notice.
    channel -> &ChannelRef = self.channel.as_ref(),

    /// ID of the channel which received this user notice.
    channel_id -> &str = self.channel_id.as_ref(),

    /// Origin of the user notice.
    ///
    /// Not available if the sender is anonymous.
    ///
    /// For example, an anonymous gift sub would be sent as a [`Event::SubGift`], but unlike many
    /// other events, there is no `AnonSubGift` variant of this one, so the [`UserNotice::sender`] field will
    /// be set to [`None`].
    sender -> Option<&User<'src>> = self.sender.as_ref(),

    /// Optional message sent along with the user notice.
    text -> Option<&str> = self.text.as_deref(),

    /// Message sent with this user notice.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    system_message -> Option<Cow<'src, str>> = self.system_message.clone().map(maybe_unescape),

    /// Event-specific information.
    event -> &Event<'src> = &self.event,

    /// ID of the event.
    ///
    /// This may be used in case it is not available as a variant of the [`Event`] enum.
    event_id -> &str = self.event_id.as_ref(),

    /// Iterator over the channel badges enabled by the user in the [channel][`UserNotice::channel`].
    badges -> impl Iterator<Item = &Badge<'src>> = self.badges.iter(),

    /// Number of channel badges enabled by the user in the [channel][`UserNotice::channel`].
    num_badges -> usize = self.badges.len(),

    /// The emote raw emote ranges present in this message.
    ///
    /// ⚠ Note: This is _hopelessly broken_ and should **never be used for any purpose whatsoever**,
    /// You should instead parse the emotes yourself out of the message according to the available emote sets.
    /// If for some reason you need it, here you go.
    raw_emotes -> &str = self.emotes.as_ref(),

    /// The user's selected name color.
    ///
    /// [`None`] means the user has not selected a color.
    /// To match the behavior of Twitch, users should be
    /// given a globally-consistent random color.
    color -> Option<&str> = self.color.as_deref(),

    /// Unique ID of the message.
    message_id -> &str = self.message_id.as_ref(),

    /// The time at which the message was sent.
    timestamp -> DateTime<Utc>,
  }
}

/// Event-specific information.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
  feature = "serde",
  derive(serde::Serialize, serde::Deserialize),
  serde(rename_all = "lowercase")
)]
pub enum Event<'src> {
  /// User subscribes or resubscribes to a channel.
  /// They are paying for their own subscription.
  #[cfg_attr(feature = "serde", serde(borrow))]
  SubOrResub(SubOrResub<'src>),

  /// The channel has been raided.
  #[cfg_attr(feature = "serde", serde(borrow))]
  Raid(Raid<'src>),

  /// A named user is gifting a subscription to a specific user.
  ///
  /// If the gift was anonymous, then [`UserNotice::sender`] will be [`None`].
  #[cfg_attr(feature = "serde", serde(borrow))]
  SubGift(SubGift<'src>),

  /// A named user is gifting a batch of subscriptions to random users.
  #[cfg_attr(feature = "serde", serde(borrow))]
  SubMysteryGift(SubMysteryGift<'src>),

  /// An anonymous user is gifting a batch of subscriptions to random users.
  #[cfg_attr(feature = "serde", serde(borrow))]
  AnonSubMysteryGift(AnonSubMysteryGift<'src>),

  /// A user continues the subscription they were gifted by a named user.
  #[cfg_attr(feature = "serde", serde(borrow))]
  GiftPaidUpgrade(GiftPaidUpgrade<'src>),

  /// A user continues the subscription they were gifted by an anonymous user.
  #[cfg_attr(feature = "serde", serde(borrow))]
  AnonGiftPaidUpgrade(AnonGiftPaidUpgrade<'src>),

  /// Rituals are automated actions.
  ///
  /// For example, the `new_chatter` ritual would consist of every chatter
  /// receiving the message:
  ///
  /// `$USER is new to $CHANNEL's chat! Say hello!`
  #[cfg_attr(feature = "serde", serde(borrow))]
  Ritual(Ritual<'src>),

  /// A user has earned a new bits badge tier.
  BitsBadgeTier(BitsBadgeTier),

  /// Someone sent an `/announcement`.
  #[cfg_attr(feature = "serde", serde(borrow))]
  Announcement(Announcement<'src>),

  #[allow(non_camel_case_types)]
  #[doc(hidden)]
  __non_exhaustive,
}

/// User subscribes or resubscribes to a channel.
/// They are paying for their own subscription.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubOrResub<'src> {
  is_resub: bool,
  cumulative_months: u64,
  streak_months: Option<u64>,
  #[cfg_attr(feature = "serde", serde(borrow))]
  sub_plan: Cow<'src, str>,
  #[cfg_attr(feature = "serde", serde(borrow))]
  sub_plan_name: Cow<'src, str>,
}

generate_getters! {
  <'src> for SubOrResub<'src> as self {
    /// If `false`, then this the user's first subscription in this channel.
    is_resub -> bool,

    /// Cumulative number of months the sending user has subscribed to this channel.
    cumulative_months -> u64,

    /// Consecutive number of months the sending user has subscribed to this channel.
    streak_months -> Option<u64>,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan -> &str = self.sub_plan.as_ref(),

    /// Channel-specific name for this subscription tier/plan.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    sub_plan_name -> Cow<'src, str> = maybe_unescape(self.sub_plan_name.clone()),
  }
}

/// The channel has been raided.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Raid<'src> {
  viewer_count: u64,
  profile_image_url: Cow<'src, str>,
}

generate_getters! {
  <'src> for Raid<'src> as self {
    /// How many viewers participated in the raid and just raided this channel.
    viewer_count -> u64,

    /// A link to the profile image of the raiding user. This is not officially documented.
    /// Empirical evidence suggests this is always the 70x70 version of the full profile
    /// picture.
    ///
    /// E.g. `https://static-cdn.jtvnw.net/jtv_user_pictures/cae3ca63-510d-4715-b4ce-059dcf938978-profile_image-70x70.png`
    profile_image_url -> &str = self.profile_image_url.as_ref(),
  }
}

/// A named user is gifting a subscription to a specific user.
///
/// If the gift was anonymous, then [`UserNotice::sender`] will be [`None`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubGift<'src> {
  cumulative_months: u64,
  recipient: User<'src>,
  sub_plan: Cow<'src, str>,
  sub_plan_name: Cow<'src, str>,
  num_gifted_months: u64,
}

generate_getters! {
  <'src> for SubGift<'src> as self {
    /// Cumulative number of months the recipient has subscribed to this channel.
    cumulative_months -> u64,

    /// The user that received this gifted subscription or resubscription.
    recipient -> User<'src>,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan -> &str = self.sub_plan.as_ref(),

    /// Channel-specific name for this subscription tier/plan.
    ///
    /// ⚠ This call will allocate and return a String if it needs to be unescaped.
    sub_plan_name -> Cow<'src, str> = maybe_unescape(self.sub_plan_name.clone()),

    /// Number of months in a single multi-month gift.
    num_gifted_months -> u64,
  }
}

/// A named user is gifting a batch of subscriptions to random users.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubMysteryGift<'src> {
  count: u64,
  sender_total_gifts: u64,
  sub_plan: Cow<'src, str>,
}

generate_getters! {
  <'src> for SubMysteryGift<'src> as self {
    /// Number of gifts.
    count -> u64,

    /// Total number of gifts the sender has gifted in this channel.
    sender_total_gifts -> u64,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan -> &str = self.sub_plan.as_ref(),
  }
}

/// An anonymous user is gifting a batch of subscriptions to random users.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnonSubMysteryGift<'src> {
  count: u64,
  sub_plan: Cow<'src, str>,
}

generate_getters! {
  <'src> for AnonSubMysteryGift<'src> as self {
    /// Number of gifts.
    count -> u64,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan -> &str = self.sub_plan.as_ref(),
  }
}

/// A user continues the subscription they were gifted by a named user.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GiftPaidUpgrade<'src> {
  gifter_login: Cow<'src, str>,
  gifter_name: Cow<'src, str>,
  promotion: Option<SubGiftPromo<'src>>,
}

generate_getters! {
  <'src> for GiftPaidUpgrade<'src> as self {
    /// Login of the gifter.
    gifter_login -> &str = self.gifter_login.as_ref(),

    /// Display name of the gifter.
    gifter_name -> &str = self.gifter_name.as_ref(),

    /// Set if the subscription is part of a promotion.
    promotion -> Option<SubGiftPromo<'src>>,
  }
}

/// A user continues the subscription they were gifted by an anonymous user.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AnonGiftPaidUpgrade<'src> {
  #[cfg_attr(feature = "serde", serde(borrow))]
  promotion: Option<SubGiftPromo<'src>>,
}

generate_getters! {
  <'src> for AnonGiftPaidUpgrade<'src> as self {
    /// Set if the subscription is part of a promotion.
    promotion -> Option<SubGiftPromo<'src>>,
  }
}

/// Rituals are automated actions.
///
/// For example, the `new_chatter` ritual would consist of every chatter
/// receiving the message:
///
/// `$USER is new to $CHANNEL's chat! Say hello!`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Ritual<'src> {
  name: Cow<'src, str>,
}

generate_getters! {
  <'src> for Ritual<'src> as self {
    /// Name of the ritual
    ///
    /// Example value: `new_chatter`
    name -> &str = self.name.as_ref(),
  }
}

/// A user has earned a new bits badge tier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BitsBadgeTier {
  /// Numeric value of the earned tier.
  ///
  /// For example, `10000` if the user earned the 10k bits badge.
  tier: u64,
}

generate_getters! {
  for BitsBadgeTier as self {
    /// Numeric value of the earned tier.
    ///
    /// For example, `10000` if the user earned the 10k bits badge.
    tier -> u64,
  }
}

/// Someone sent an `/announcement`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Announcement<'src> {
  highlight_color: Cow<'src, str>,
}

generate_getters! {
  <'src> for Announcement<'src> as self {
    /// The color used to highlight the announcement.
    ///
    /// Currently, the possible values are:
    /// - `PRIMARY`
    /// - `BLUE`
    /// - `GREEN`
    /// - `ORANGE`
    /// - `PURPLE`
    ///
    /// Where `PRIMARY` refers to the channel's profile accent color.
    highlight_color -> &str = self.highlight_color.as_ref(),
  }
}

/// Used in [`Event::GiftPaidUpgrade`] and [`Event::AnonGiftPaidUpgrade`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubGiftPromo<'src> {
  total_gifts: u64,
  promo_name: Cow<'src, str>,
}

generate_getters! {
  <'src> for SubGiftPromo<'src> as self {
    /// Total number of subs gifted during this promotion
    total_gifts -> u64,

    /// Display of the promotion, e.g. `Subtember 2018`
    promo_name -> &str = self.promo_name.as_ref(),
  }
}

fn parse_promotion<'src>(message: &IrcMessageRef<'src>) -> Option<SubGiftPromo<'src>> {
  match (
    message
      .tag(Tag::MsgParamPromoGiftTotal)
      .and_then(|v| v.parse().ok()),
    message.tag(Tag::MsgParamPromoName),
  ) {
    (Some(total_gifts), Some(promo_name)) => Some(SubGiftPromo {
      total_gifts,
      promo_name: promo_name.into(),
    }),
    _ => None,
  }
}

/// Some events are sent with this specific sender ID.
/// If it is present, then the event is anonymous.
const AN_ANONYMOUS_GIFTER: Option<&str> = Some("274598607");

impl<'src> UserNotice<'src> {
  fn parse(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::UserNotice {
      return None;
    }

    let sender_id = message.tag(Tag::UserId);
    let event_id = message.tag(Tag::MsgId)?;
    let (event, is_anon) = match event_id {
      "sub" | "resub" => (
        Event::SubOrResub(SubOrResub {
          is_resub: event_id == "resub",
          cumulative_months: message
            .tag(Tag::MsgParamCumulativeMonths)
            .and_then(|v| v.parse().ok())?,
          streak_months: message
            .tag(Tag::MsgParamStreakMonths)
            .and_then(|v| v.parse().ok())
            .and_then(|n| if n > 0 { Some(n) } else { None }),
          sub_plan: message.tag(Tag::MsgParamSubPlan)?.into(),
          sub_plan_name: message.tag(Tag::MsgParamSubPlanName)?.into(),
        }),
        false,
      ),
      "raid" => (
        Event::Raid(Raid {
          viewer_count: message
            .tag(Tag::MsgParamViewerCount)
            .and_then(|v| v.parse().ok())?,
          profile_image_url: message.tag(Tag::MsgParamProfileImageUrl)?.into(),
        }),
        false,
      ),
      "subgift" | "anonsubgift" => (
        Event::SubGift(SubGift {
          cumulative_months: message
            .tag(Tag::MsgParamMonths)
            .and_then(|v| v.parse().ok())?,
          recipient: User {
            id: message.tag(Tag::MsgParamRecipientId)?.into(),
            login: message.tag(Tag::MsgParamRecipientUserName)?.into(),
            name: message.tag(Tag::MsgParamRecipientDisplayName)?.into(),
          },
          sub_plan: message.tag(Tag::MsgParamSubPlan)?.into(),
          sub_plan_name: message.tag(Tag::MsgParamSubPlanName)?.into(),
          num_gifted_months: message
            .tag(Tag::MsgParamGiftMonths)
            .and_then(|v| v.parse().ok())?,
        }),
        event_id == "anonsubgift" || sender_id == AN_ANONYMOUS_GIFTER,
      ),
      "anonsubmysterygift" => (
        Event::AnonSubMysteryGift(AnonSubMysteryGift {
          count: message
            .tag(Tag::MsgParamMassGiftCount)
            .and_then(|v| v.parse().ok())?,
          sub_plan: message.tag(Tag::MsgParamSubPlan)?.into(),
        }),
        true,
      ),
      "submysterygift" if sender_id == AN_ANONYMOUS_GIFTER => (
        Event::AnonSubMysteryGift(AnonSubMysteryGift {
          count: message
            .tag(Tag::MsgParamMassGiftCount)
            .and_then(|v| v.parse().ok())?,
          sub_plan: message.tag(Tag::MsgParamSubPlan)?.into(),
        }),
        true,
      ),
      "submysterygift" => (
        Event::SubMysteryGift(SubMysteryGift {
          count: message
            .tag(Tag::MsgParamMassGiftCount)
            .and_then(|v| v.parse().ok())?,
          sender_total_gifts: message
            .tag(Tag::MsgParamSenderCount)
            .and_then(|v| v.parse().ok())?,
          sub_plan: message.tag(Tag::MsgParamSubPlan)?.into(),
        }),
        false,
      ),
      "giftpaidupgrade" => (
        Event::GiftPaidUpgrade(GiftPaidUpgrade {
          gifter_login: message.tag(Tag::MsgParamSenderLogin)?.into(),
          gifter_name: message.tag(Tag::MsgParamSenderName)?.into(),
          promotion: parse_promotion(&message),
        }),
        false,
      ),
      "anongiftpaidupgrade" => (
        Event::AnonGiftPaidUpgrade(AnonGiftPaidUpgrade {
          promotion: parse_promotion(&message),
        }),
        true,
      ),
      "ritual" => (
        Event::Ritual(Ritual {
          name: message.tag(Tag::MsgParamRitualName)?.into(),
        }),
        false,
      ),
      "bitsbadgetier" => (
        Event::BitsBadgeTier(BitsBadgeTier {
          tier: message
            .tag(Tag::MsgParamThreshold)
            .and_then(|v| v.parse().ok())?,
        }),
        false,
      ),
      "announcement" => (
        Event::Announcement(Announcement {
          highlight_color: message.tag(Tag::MsgParamColor)?.into(),
        }),
        false,
      ),
      _ => (Event::__non_exhaustive, true),
    };

    let sender = if !is_anon {
      Some(User {
        id: message.tag(Tag::UserId)?.into(),
        login: message.tag(Tag::Login)?.into(),
        name: message.tag(Tag::DisplayName)?.into(),
      })
    } else {
      None
    };

    Some(UserNotice {
      channel: MaybeOwned::Ref(message.channel()?),
      channel_id: message.tag(Tag::RoomId)?.into(),
      sender,
      text: message.text().map(Cow::Borrowed),
      system_message: message
        .tag(Tag::SystemMsg)
        .filter(is_not_empty)
        .map(Cow::Borrowed),
      event,
      event_id: event_id.into(),
      badges: message
        .tag(Tag::Badges)
        .zip(message.tag(Tag::BadgeInfo))
        .map(|(badges, badge_info)| parse_badges(badges, badge_info))
        .unwrap_or_default(),
      emotes: message.tag(Tag::Emotes).unwrap_or_default().into(),
      color: message
        .tag(Tag::Color)
        .filter(is_not_empty)
        .map(Cow::Borrowed),
      message_id: message.tag(Tag::Id)?.into(),
      timestamp: message.tag(Tag::TmiSentTs).and_then(parse_timestamp)?,
    })
  }
}

impl<'src> super::FromIrc<'src> for UserNotice<'src> {
  #[inline]
  fn from_irc(message: IrcMessageRef<'src>) -> Result<Self, MessageParseError> {
    Self::parse(message).ok_or(MessageParseError)
  }
}

impl<'src> From<UserNotice<'src>> for super::Message<'src> {
  fn from(msg: UserNotice<'src>) -> Self {
    super::Message::UserNotice(msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_user_notice_announcement() {
    assert_irc_snapshot!(UserNotice, "@emotes=;login=pajbot;vip=0;tmi-sent-ts=1695554663565;flags=;mod=1;subscriber=1;id=bb1bec25-8f26-4ba3-a084-a6a2ca332f00;badge-info=subscriber/93;system-msg=;user-id=82008718;user-type=mod;room-id=11148817;badges=moderator/1,subscriber/3072;msg-param-color=PRIMARY;msg-id=announcement;color=#2E8B57;display-name=pajbot :tmi.twitch.tv USERNOTICE #pajlada :$ping xd");
  }

  #[test]
  fn parse_sub() {
    assert_irc_snapshot!(UserNotice, "@badge-info=subscriber/0;badges=subscriber/0,premium/1;color=;display-name=fallenseraphhh;emotes=;flags=;id=2a9bea11-a80a-49a0-a498-1642d457f775;login=fallenseraphhh;mod=0;msg-id=sub;msg-param-cumulative-months=1;msg-param-months=0;msg-param-should-share-streak=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=Prime;room-id=71092938;subscriber=1;system-msg=fallenseraphhh\\ssubscribed\\swith\\sTwitch\\sPrime.;tmi-sent-ts=1582685713242;user-id=224005980;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_resub() {
    assert_irc_snapshot!(UserNotice, "@badge-info=subscriber/2;badges=subscriber/0,battlerite_1/1;color=#0000FF;display-name=Gutrin;emotes=1035663:0-3;flags=;id=e0975c76-054c-4954-8cb0-91b8867ec1ca;login=gutrin;mod=0;msg-id=resub;msg-param-cumulative-months=2;msg-param-months=0;msg-param-should-share-streak=1;msg-param-streak-months=2;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=1;system-msg=Gutrin\\ssubscribed\\sat\\sTier\\s1.\\sThey've\\ssubscribed\\sfor\\s2\\smonths,\\scurrently\\son\\sa\\s2\\smonth\\sstreak!;tmi-sent-ts=1581713640019;user-id=21156217;user-type= :tmi.twitch.tv USERNOTICE #xqcow :xqcL");
  }

  #[test]
  fn parse_resub_no_share_streak() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=premium/1;color=#8A2BE2;display-name=rene_rs;emotes=;flags=;id=ca1f02fb-77ec-487d-a9b3-bc4bfef2fe8b;login=rene_rs;mod=0;msg-id=resub;msg-param-cumulative-months=11;msg-param-months=0;msg-param-should-share-streak=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=Prime;room-id=71092938;subscriber=0;system-msg=rene_rs\\ssubscribed\\swith\\sTwitch\\sPrime.\\sThey've\\ssubscribed\\sfor\\s11\\smonths!;tmi-sent-ts=1590628650446;user-id=171356987;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_raid() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=sub-gifter/50;color=;display-name=AdamAtReflectStudios;emotes=;flags=;id=e21409b1-d25d-4a1a-b5cf-ef27d8b7030e;login=adamatreflectstudios;mod=0;msg-id=subgift;msg-param-gift-months=1;msg-param-months=2;msg-param-origin-id=da\\s39\\sa3\\see\\s5e\\s6b\\s4b\\s0d\\s32\\s55\\sbf\\sef\\s95\\s60\\s18\\s90\\saf\\sd8\\s07\\s09;msg-param-recipient-display-name=qatarking24xd;msg-param-recipient-id=236653628;msg-param-recipient-user-name=qatarking24xd;msg-param-sender-count=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=AdamAtReflectStudios\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sqatarking24xd!;tmi-sent-ts=1594583782376;user-id=211711554;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_subgift_ananonymousgifter() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=;color=;display-name=AnAnonymousGifter;emotes=;flags=;id=62c3fd39-84cc-452a-9096-628a5306633a;login=ananonymousgifter;mod=0;msg-id=subgift;msg-param-fun-string=FunStringThree;msg-param-gift-months=1;msg-param-months=13;msg-param-origin-id=da\\s39\\sa3\\see\\s5e\\s6b\\s4b\\s0d\\s32\\s55\\sbf\\sef\\s95\\s60\\s18\\s90\\saf\\sd8\\s07\\s09;msg-param-recipient-display-name=Dot0422;msg-param-recipient-id=151784015;msg-param-recipient-user-name=dot0422;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=An\\sanonymous\\suser\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sDot0422!\\s;tmi-sent-ts=1594495108936;user-id=274598607;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_anonsubgift() {
    // note there are no anonsubgift messages being sent on Twitch IRC as of the time of writing this.
    // so I created a fake one that matches what the announcement said they would be like (in theory),

    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=;color=;display-name=xQcOW;emotes=;flags=;id=e21409b1-d25d-4a1a-b5cf-ef27d8b7030e;login=xqcow;mod=0;msg-id=anonsubgift;msg-param-gift-months=1;msg-param-months=2;msg-param-origin-id=da\\s39\\sa3\\see\\s5e\\s6b\\s4b\\s0d\\s32\\s55\\sbf\\sef\\s95\\s60\\s18\\s90\\saf\\sd8\\s07\\s09;msg-param-recipient-display-name=qatarking24xd;msg-param-recipient-id=236653628;msg-param-recipient-user-name=qatarking24xd;msg-param-sender-count=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=An\\sanonymous\\sgifter\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sqatarking24xd!;tmi-sent-ts=1594583782376;user-id=71092938;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_submysterygift() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=sub-gifter/50;color=;display-name=AdamAtReflectStudios;emotes=;flags=;id=049e6371-7023-4fca-8605-7dec60e72e12;login=adamatreflectstudios;mod=0;msg-id=submysterygift;msg-param-mass-gift-count=20;msg-param-origin-id=1f\\sbe\\sbb\\s4a\\s81\\s9a\\s65\\sd1\\s4b\\s77\\sf5\\s23\\s16\\s4a\\sd3\\s13\\s09\\se7\\sbe\\s55;msg-param-sender-count=100;msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=AdamAtReflectStudios\\sis\\sgifting\\s20\\sTier\\s1\\sSubs\\sto\\sxQcOW's\\scommunity!\\sThey've\\sgifted\\sa\\stotal\\sof\\s100\\sin\\sthe\\schannel!;tmi-sent-ts=1594583777669;user-id=211711554;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_submysterygift_ananonymousgifter() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=;color=;display-name=AnAnonymousGifter;emotes=;flags=;id=8db97752-3dee-460b-9001-e925d0e2ba5b;login=ananonymousgifter;mod=0;msg-id=submysterygift;msg-param-mass-gift-count=10;msg-param-origin-id=13\\s33\\sed\\sc0\\sef\\sa0\\s7b\\s9b\\s48\\s59\\scb\\scc\\se4\\s39\\s7b\\s90\\sf9\\s54\\s75\\s66;msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=An\\sanonymous\\suser\\sis\\sgifting\\s10\\sTier\\s1\\sSubs\\sto\\sxQcOW's\\scommunity!;tmi-sent-ts=1585447099603;user-id=274598607;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_anonsubmysterygift() {
    // again, this is never emitted on IRC currently. So this test case is a made-up
    // modification of a subgift type message.

    assert_irc_snapshot!(UserNotice, "@badge-info=subscriber/2;badges=subscriber/2;color=#00FFF5;display-name=CrazyCrackAnimal;emotes=;flags=;id=7006f242-a45c-4e07-83b3-11f9c6d1ee28;login=crazycrackanimal;mod=0;msg-id=giftpaidupgrade;msg-param-sender-login=stridezgum;msg-param-sender-name=Stridezgum;room-id=71092938;subscriber=1;system-msg=CrazyCrackAnimal\\sis\\scontinuing\\sthe\\sGift\\sSub\\sthey\\sgot\\sfrom\\sStridezgum!;tmi-sent-ts=1594518849459;user-id=86082877;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_giftpaidupgrade_with_promo() {
    // I can't find any real examples for this type of message, so this is a made-up test case
    // (the same one as above, but with two tags added)

    assert_irc_snapshot!(UserNotice, "@badge-info=subscriber/1;badges=subscriber/0,premium/1;color=#8A2BE2;display-name=samura1jack_ttv;emotes=;flags=;id=144ee636-0c1d-404e-8b29-35449a045a7e;login=samura1jack_ttv;mod=0;msg-id=anongiftpaidupgrade;room-id=71092938;subscriber=1;system-msg=samura1jack_ttv\\sis\\scontinuing\\sthe\\sGift\\sSub\\sthey\\sgot\\sfrom\\san\\sanonymous\\suser!;tmi-sent-ts=1594327421732;user-id=102707709;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_anongiftpaidupgrade_with_promo() {
    // I can't find any real examples for this type of message, so this is a made-up test case
    // (the same one as above, but with two tags added)

    assert_irc_snapshot!(UserNotice, "@badge-info=subscriber/1;badges=subscriber/0,premium/1;color=#8A2BE2;display-name=samura1jack_ttv;emotes=;flags=;id=144ee636-0c1d-404e-8b29-35449a045a7e;msg-param-promo-name=TestSubtember2020;msg-param-promo-gift-total=4003;login=samura1jack_ttv;mod=0;msg-id=anongiftpaidupgrade;room-id=71092938;subscriber=1;system-msg=samura1jack_ttv\\sis\\scontinuing\\sthe\\sGift\\sSub\\sthey\\sgot\\sfrom\\san\\sanonymous\\suser!\\sbla\\sbla\\sbla\\sstuff\\sabout\\spromo\\shere;tmi-sent-ts=1594327421732;user-id=102707709;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[test]
  fn parse_ritual() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=;color=;display-name=SevenTest1;emotes=30259:0-6;id=37feed0f-b9c7-4c3a-b475-21c6c6d21c3d;login=seventest1;mod=0;msg-id=ritual;msg-param-ritual-name=new_chatter;room-id=6316121;subscriber=0;system-msg=Seventoes\\sis\\snew\\shere!;tmi-sent-ts=1508363903826;turbo=0;user-id=131260580;user-type= :tmi.twitch.tv USERNOTICE #seventoes :HeyGuys");
  }

  #[test]
  fn parse_bitsbadgetier() {
    assert_irc_snapshot!(UserNotice, "@badge-info=;badges=sub-gifter/50;color=;display-name=AdamAtReflectStudios;emotes=;flags=;id=7f1336e4-f84a-4510-809d-e57bf50af0cc;login=adamatreflectstudios;mod=0;msg-id=rewardgift;msg-param-domain=pride_megacommerce_2020;msg-param-selected-count=100;msg-param-total-reward-count=100;msg-param-trigger-amount=20;msg-param-trigger-type=SUBGIFT;room-id=71092938;subscriber=0;system-msg=AdamAtReflectStudios's\\sGift\\sshared\\srewards\\sto\\s100\\sothers\\sin\\sChat!;tmi-sent-ts=1594583778756;user-id=211711554;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_user_notice_announcement() {
    assert_irc_roundtrip!(UserNotice, "@emotes=;login=pajbot;vip=0;tmi-sent-ts=1695554663565;flags=;mod=1;subscriber=1;id=bb1bec25-8f26-4ba3-a084-a6a2ca332f00;badge-info=subscriber/93;system-msg=;user-id=82008718;user-type=mod;room-id=11148817;badges=moderator/1,subscriber/3072;msg-param-color=PRIMARY;msg-id=announcement;color=#2E8B57;display-name=pajbot :tmi.twitch.tv USERNOTICE #pajlada :$ping xd");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_sub() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=subscriber/0;badges=subscriber/0,premium/1;color=;display-name=fallenseraphhh;emotes=;flags=;id=2a9bea11-a80a-49a0-a498-1642d457f775;login=fallenseraphhh;mod=0;msg-id=sub;msg-param-cumulative-months=1;msg-param-months=0;msg-param-should-share-streak=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=Prime;room-id=71092938;subscriber=1;system-msg=fallenseraphhh\\ssubscribed\\swith\\sTwitch\\sPrime.;tmi-sent-ts=1582685713242;user-id=224005980;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_resub() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=subscriber/2;badges=subscriber/0,battlerite_1/1;color=#0000FF;display-name=Gutrin;emotes=1035663:0-3;flags=;id=e0975c76-054c-4954-8cb0-91b8867ec1ca;login=gutrin;mod=0;msg-id=resub;msg-param-cumulative-months=2;msg-param-months=0;msg-param-should-share-streak=1;msg-param-streak-months=2;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=1;system-msg=Gutrin\\ssubscribed\\sat\\sTier\\s1.\\sThey've\\ssubscribed\\sfor\\s2\\smonths,\\scurrently\\son\\sa\\s2\\smonth\\sstreak!;tmi-sent-ts=1581713640019;user-id=21156217;user-type= :tmi.twitch.tv USERNOTICE #xqcow :xqcL");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_resub_no_share_streak() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=premium/1;color=#8A2BE2;display-name=rene_rs;emotes=;flags=;id=ca1f02fb-77ec-487d-a9b3-bc4bfef2fe8b;login=rene_rs;mod=0;msg-id=resub;msg-param-cumulative-months=11;msg-param-months=0;msg-param-should-share-streak=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=Prime;room-id=71092938;subscriber=0;system-msg=rene_rs\\ssubscribed\\swith\\sTwitch\\sPrime.\\sThey've\\ssubscribed\\sfor\\s11\\smonths!;tmi-sent-ts=1590628650446;user-id=171356987;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_raid() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=sub-gifter/50;color=;display-name=AdamAtReflectStudios;emotes=;flags=;id=e21409b1-d25d-4a1a-b5cf-ef27d8b7030e;login=adamatreflectstudios;mod=0;msg-id=subgift;msg-param-gift-months=1;msg-param-months=2;msg-param-origin-id=da\\s39\\sa3\\see\\s5e\\s6b\\s4b\\s0d\\s32\\s55\\sbf\\sef\\s95\\s60\\s18\\s90\\saf\\sd8\\s07\\s09;msg-param-recipient-display-name=qatarking24xd;msg-param-recipient-id=236653628;msg-param-recipient-user-name=qatarking24xd;msg-param-sender-count=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=AdamAtReflectStudios\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sqatarking24xd!;tmi-sent-ts=1594583782376;user-id=211711554;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_subgift_ananonymousgifter() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=;color=;display-name=AnAnonymousGifter;emotes=;flags=;id=62c3fd39-84cc-452a-9096-628a5306633a;login=ananonymousgifter;mod=0;msg-id=subgift;msg-param-fun-string=FunStringThree;msg-param-gift-months=1;msg-param-months=13;msg-param-origin-id=da\\s39\\sa3\\see\\s5e\\s6b\\s4b\\s0d\\s32\\s55\\sbf\\sef\\s95\\s60\\s18\\s90\\saf\\sd8\\s07\\s09;msg-param-recipient-display-name=Dot0422;msg-param-recipient-id=151784015;msg-param-recipient-user-name=dot0422;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=An\\sanonymous\\suser\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sDot0422!\\s;tmi-sent-ts=1594495108936;user-id=274598607;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_anonsubgift() {
    // note there are no anonsubgift messages being sent on Twitch IRC as of the time of writing this.
    // so I created a fake one that matches what the announcement said they would be like (in theory),

    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=;color=;display-name=xQcOW;emotes=;flags=;id=e21409b1-d25d-4a1a-b5cf-ef27d8b7030e;login=xqcow;mod=0;msg-id=anonsubgift;msg-param-gift-months=1;msg-param-months=2;msg-param-origin-id=da\\s39\\sa3\\see\\s5e\\s6b\\s4b\\s0d\\s32\\s55\\sbf\\sef\\s95\\s60\\s18\\s90\\saf\\sd8\\s07\\s09;msg-param-recipient-display-name=qatarking24xd;msg-param-recipient-id=236653628;msg-param-recipient-user-name=qatarking24xd;msg-param-sender-count=0;msg-param-sub-plan-name=Channel\\sSubscription\\s(xqcow);msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=An\\sanonymous\\sgifter\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sqatarking24xd!;tmi-sent-ts=1594583782376;user-id=71092938;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_submysterygift() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=sub-gifter/50;color=;display-name=AdamAtReflectStudios;emotes=;flags=;id=049e6371-7023-4fca-8605-7dec60e72e12;login=adamatreflectstudios;mod=0;msg-id=submysterygift;msg-param-mass-gift-count=20;msg-param-origin-id=1f\\sbe\\sbb\\s4a\\s81\\s9a\\s65\\sd1\\s4b\\s77\\sf5\\s23\\s16\\s4a\\sd3\\s13\\s09\\se7\\sbe\\s55;msg-param-sender-count=100;msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=AdamAtReflectStudios\\sis\\sgifting\\s20\\sTier\\s1\\sSubs\\sto\\sxQcOW's\\scommunity!\\sThey've\\sgifted\\sa\\stotal\\sof\\s100\\sin\\sthe\\schannel!;tmi-sent-ts=1594583777669;user-id=211711554;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_submysterygift_ananonymousgifter() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=;color=;display-name=AnAnonymousGifter;emotes=;flags=;id=8db97752-3dee-460b-9001-e925d0e2ba5b;login=ananonymousgifter;mod=0;msg-id=submysterygift;msg-param-mass-gift-count=10;msg-param-origin-id=13\\s33\\sed\\sc0\\sef\\sa0\\s7b\\s9b\\s48\\s59\\scb\\scc\\se4\\s39\\s7b\\s90\\sf9\\s54\\s75\\s66;msg-param-sub-plan=1000;room-id=71092938;subscriber=0;system-msg=An\\sanonymous\\suser\\sis\\sgifting\\s10\\sTier\\s1\\sSubs\\sto\\sxQcOW's\\scommunity!;tmi-sent-ts=1585447099603;user-id=274598607;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_anonsubmysterygift() {
    // again, this is never emitted on IRC currently. So this test case is a made-up
    // modification of a subgift type message.

    assert_irc_roundtrip!(UserNotice, "@badge-info=subscriber/2;badges=subscriber/2;color=#00FFF5;display-name=CrazyCrackAnimal;emotes=;flags=;id=7006f242-a45c-4e07-83b3-11f9c6d1ee28;login=crazycrackanimal;mod=0;msg-id=giftpaidupgrade;msg-param-sender-login=stridezgum;msg-param-sender-name=Stridezgum;room-id=71092938;subscriber=1;system-msg=CrazyCrackAnimal\\sis\\scontinuing\\sthe\\sGift\\sSub\\sthey\\sgot\\sfrom\\sStridezgum!;tmi-sent-ts=1594518849459;user-id=86082877;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_giftpaidupgrade_with_promo() {
    // I can't find any real examples for this type of message, so this is a made-up test case
    // (the same one as above, but with two tags added)

    assert_irc_roundtrip!(UserNotice, "@badge-info=subscriber/1;badges=subscriber/0,premium/1;color=#8A2BE2;display-name=samura1jack_ttv;emotes=;flags=;id=144ee636-0c1d-404e-8b29-35449a045a7e;login=samura1jack_ttv;mod=0;msg-id=anongiftpaidupgrade;room-id=71092938;subscriber=1;system-msg=samura1jack_ttv\\sis\\scontinuing\\sthe\\sGift\\sSub\\sthey\\sgot\\sfrom\\san\\sanonymous\\suser!;tmi-sent-ts=1594327421732;user-id=102707709;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_anongiftpaidupgrade_with_promo() {
    // I can't find any real examples for this type of message, so this is a made-up test case
    // (the same one as above, but with two tags added)

    assert_irc_roundtrip!(UserNotice, "@badge-info=subscriber/1;badges=subscriber/0,premium/1;color=#8A2BE2;display-name=samura1jack_ttv;emotes=;flags=;id=144ee636-0c1d-404e-8b29-35449a045a7e;msg-param-promo-name=TestSubtember2020;msg-param-promo-gift-total=4003;login=samura1jack_ttv;mod=0;msg-id=anongiftpaidupgrade;room-id=71092938;subscriber=1;system-msg=samura1jack_ttv\\sis\\scontinuing\\sthe\\sGift\\sSub\\sthey\\sgot\\sfrom\\san\\sanonymous\\suser!\\sbla\\sbla\\sbla\\sstuff\\sabout\\spromo\\shere;tmi-sent-ts=1594327421732;user-id=102707709;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_ritual() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=;color=;display-name=SevenTest1;emotes=30259:0-6;id=37feed0f-b9c7-4c3a-b475-21c6c6d21c3d;login=seventest1;mod=0;msg-id=ritual;msg-param-ritual-name=new_chatter;room-id=6316121;subscriber=0;system-msg=Seventoes\\sis\\snew\\shere!;tmi-sent-ts=1508363903826;turbo=0;user-id=131260580;user-type= :tmi.twitch.tv USERNOTICE #seventoes :HeyGuys");
  }

  #[cfg(feature = "serde")]
  #[test]
  fn roundtrip_bitsbadgetier() {
    assert_irc_roundtrip!(UserNotice, "@badge-info=;badges=sub-gifter/50;color=;display-name=AdamAtReflectStudios;emotes=;flags=;id=7f1336e4-f84a-4510-809d-e57bf50af0cc;login=adamatreflectstudios;mod=0;msg-id=rewardgift;msg-param-domain=pride_megacommerce_2020;msg-param-selected-count=100;msg-param-total-reward-count=100;msg-param-trigger-amount=20;msg-param-trigger-type=SUBGIFT;room-id=71092938;subscriber=0;system-msg=AdamAtReflectStudios's\\sGift\\sshared\\srewards\\sto\\s100\\sothers\\sin\\sChat!;tmi-sent-ts=1594583778756;user-id=211711554;user-type= :tmi.twitch.tv USERNOTICE #xqcow");
  }
}
