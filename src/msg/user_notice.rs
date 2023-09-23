use super::{parse_badges, parse_emotes, parse_timestamp, Badge, Emote, SmallVec, User};
use crate::{Command, IrcMessageRef, Tag};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct UserNotice<'src> {
  /// Name of the channel which received this user notice.
  pub channel: &'src str,

  /// ID of the channel which received this user notice.
  pub channel_id: &'src str,

  /// Origin of the user notice.
  ///
  /// Not available if the sender is anonymous.
  ///
  /// For example, an anonymous gift sub would be sent as a [`UserNoticeEvent::SubGift`], but unlike many
  /// other events, there is no `AnonSubGift` variant of this one, so the [`UserNotice::sender`] field will
  /// be set to [`None`].
  pub sender: Option<User<'src>>,

  /// Optional message sent along with the user notice.
  pub text: Option<&'src str>,

  /// Message sent with this user notice.
  pub system_message: &'src str,

  /// Event-specific information.
  pub event: UserNoticeEvent<'src>,

  /// ID of the event.
  ///
  /// This may be used in case it is not available as a variant of the [`UserNoticeEvent`] enum.
  pub event_id: &'src str,

  /// List of channel badges enabled by the user in the [channel][`UserNotice::channel`].
  pub badges: SmallVec<Badge<'src>, 2>,

  /// The emote ranges present in this message.
  pub emotes: Vec<Emote<'src>>,

  /// The user's selected name color.
  ///
  /// [`None`] means the user has not selected a color.
  /// To match the behavior of Twitch, users should be
  /// given a globally-consistent random color.
  pub color: Option<&'src str>,

  /// Unique ID of the message.
  pub message_id: &'src str,

  /// The time at which the message was sent.
  pub timestamp: DateTime<Utc>,
}

/// Event-specific information.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum UserNoticeEvent<'src> {
  /// User subscribes or resubscribes to a channel.
  /// They are paying for their own subscription.
  SubOrResub {
    /// If `false`, then this the user's first subscription in this channel.
    is_resub: bool,

    /// Cumulative number of months the sending user has subscribed to this channel.
    cumulative_months: u64,

    /// Consecutive number of months the sending user has subscribed to this channel.
    streak_months: Option<u64>,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan: &'src str,

    /// Channel-specific name for this subscription tier/plan.
    sub_plan_name: &'src str,
  },

  /// The channel has been raided.
  Raid {
    /// How many viewers participated in the raid and just raided this channel.
    viewer_count: u64,

    /// A link to the profile image of the raiding user. This is not officially documented.
    /// Empirical evidence suggests this is always the 70x70 version of the full profile
    /// picture.
    ///
    /// E.g. `https://static-cdn.jtvnw.net/jtv_user_pictures/cae3ca63-510d-4715-b4ce-059dcf938978-profile_image-70x70.png`
    profile_image_url: &'src str,
  },

  /// A named user is gifting a subscription to a specific user.
  ///
  /// If the gift was anonymous, then [`UserNotice::sender`] will be [`None`].
  SubGift {
    /// Cumulative number of months the recipient has subscribed to this channel.
    cumulative_months: u64,

    /// The user that received this gifted subscription or resubscription.
    recipient: User<'src>,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan: &'src str,

    /// Channel-specific name for this subscription tier/plan.
    sub_plan_name: &'src str,

    /// Number of months in a single multi-month gift.
    num_gifted_months: u64,
  },

  /// A named user is gifting a batch of subscriptions to random users.
  SubMysteryGift {
    /// Number of gifts.
    count: u64,

    /// Total number of gifts the sender has gifted in this channel.
    sender_total_gifts: u64,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan: &'src str,
  },

  /// An anonymous user is gifting a batch of subscriptions to random users.
  AnonSubMysteryGift {
    /// Number of gifts.
    count: u64,

    /// Subcription tier/plan.
    /// For example:
    /// - `Prime` -> Twitch Prime
    /// - `1000` -> Tier 1
    /// - `2000` -> Tier 2
    /// - `3000` -> Tier 3
    sub_plan: &'src str,
  },

  /// A user continues the subscription they were gifted by a named user.
  GiftPaidUpgrade {
    /// Login of the gifter.
    gifter_login: &'src str,

    /// Display name of the gifter.
    gifter_name: &'src str,

    /// Set if the subscription is part of a promotion.
    promotion: Option<SubGiftPromo<'src>>,
  },

  /// A user continues the subscription they were gifted by an anonymous user.
  AnonGiftPaidUpgrade {
    /// Set if the subscription is part of a promotion.
    promotion: Option<SubGiftPromo<'src>>,
  },

  /// Rituals are automated actions.
  ///
  /// For example, the `new_chatter` ritual would consist of every chatter
  /// receiving the message:
  ///
  /// `$USER is new to $CHANNEL's chat! Say hello!`
  Ritual {
    /// Name of the ritual
    ///
    /// Example value: `new_chatter`
    name: &'src str,
  },

  /// Sent when a user earns a new bits badge tier.
  BitsBadgeTier {
    /// Numeric value of the earned tier.
    ///
    /// For example, `10000` if the user earned the 10k bits badge.
    tier: u64,
  },

  #[allow(non_camel_case_types)]
  #[doc(hidden)]
  __non_exhaustive,
}

/// Used in [`UserNoticeEvent::GiftPaidUpgrade`] and [`UserNoticeEvent::AnonGiftPaidUpgrade`].
#[derive(Clone, Debug)]
pub struct SubGiftPromo<'src> {
  /// Total number of subs gifted during this promotion
  pub total_gifts: u64,

  /// Display of the promotion, e.g. `Subtember 2018`
  pub promo_name: &'src str,
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
      promo_name,
    }),
    _ => None,
  }
}

/// Some events are sent with this specific sender ID.
/// If it is present, then the event is anonymous.
const AN_ANONYMOUS_GIFTER: Option<&str> = Some("274598607");

impl<'src> super::FromIrc<'src> for UserNotice<'src> {
  fn from_irc(message: IrcMessageRef<'src>) -> Option<Self> {
    if message.command() != Command::UserNotice {
      return None;
    }

    let sender_id = message.tag(Tag::UserId);
    let event_id = message.tag(Tag::MsgId)?;
    let (event, is_anon) = match event_id {
      "sub" | "resub" => (
        UserNoticeEvent::SubOrResub {
          is_resub: event_id == "resub",
          cumulative_months: message
            .tag(Tag::MsgParamCumulativeMonths)
            .and_then(|v| v.parse().ok())?,
          streak_months: message
            .tag(Tag::MsgParamStreakMonths)
            .and_then(|v| v.parse().ok())
            .and_then(|n| if n > 0 { Some(n) } else { None }),
          sub_plan: message.tag(Tag::MsgParamSubPlan)?,
          sub_plan_name: message.tag(Tag::MsgParamSubPlanName)?,
        },
        false,
      ),
      "raid" => (
        UserNoticeEvent::Raid {
          viewer_count: message
            .tag(Tag::MsgParamViewerCount)
            .and_then(|v| v.parse().ok())?,
          profile_image_url: message.tag(Tag::MsgParamProfileImageUrl)?,
        },
        false,
      ),
      "subgift" | "anonsubgift" => (
        UserNoticeEvent::SubGift {
          cumulative_months: message
            .tag(Tag::MsgParamMonths)
            .and_then(|v| v.parse().ok())?,
          recipient: User {
            id: message.tag(Tag::MsgParamRecipientId)?,
            login: message.tag(Tag::MsgParamRecipientUserName)?,
            name: message.tag(Tag::MsgParamRecipientDisplayName)?,
          },
          sub_plan: message.tag(Tag::MsgParamSubPlan)?,
          sub_plan_name: message.tag(Tag::MsgParamSubPlanName)?,
          num_gifted_months: message
            .tag(Tag::MsgParamGiftMonths)
            .and_then(|v| v.parse().ok())?,
        },
        event_id == "anonsubgift" || sender_id == AN_ANONYMOUS_GIFTER,
      ),
      "anonsubmysterygift" => (
        UserNoticeEvent::AnonSubMysteryGift {
          count: message
            .tag(Tag::MsgParamMassGiftCount)
            .and_then(|v| v.parse().ok())?,
          sub_plan: message.tag(Tag::MsgParamSubPlan)?,
        },
        true,
      ),
      "submysterygift" if sender_id == AN_ANONYMOUS_GIFTER => (
        UserNoticeEvent::AnonSubMysteryGift {
          count: message
            .tag(Tag::MsgParamMassGiftCount)
            .and_then(|v| v.parse().ok())?,
          sub_plan: message.tag(Tag::MsgParamSubPlan)?,
        },
        true,
      ),
      "submysterygift" => (
        UserNoticeEvent::SubMysteryGift {
          count: message
            .tag(Tag::MsgParamMassGiftCount)
            .and_then(|v| v.parse().ok())?,
          sender_total_gifts: message
            .tag(Tag::MsgParamSenderCount)
            .and_then(|v| v.parse().ok())?,
          sub_plan: message.tag(Tag::MsgParamSubPlan)?,
        },
        false,
      ),
      "giftpaidupgrade" => (
        UserNoticeEvent::GiftPaidUpgrade {
          gifter_login: message.tag(Tag::MsgParamSenderLogin)?,
          gifter_name: message.tag(Tag::MsgParamSenderName)?,
          promotion: parse_promotion(&message),
        },
        false,
      ),
      "anongiftpaidupgrade" => (
        UserNoticeEvent::AnonGiftPaidUpgrade {
          promotion: parse_promotion(&message),
        },
        true,
      ),
      "ritual" => (
        UserNoticeEvent::Ritual {
          name: message.tag(Tag::MsgParamRitualName)?,
        },
        false,
      ),
      "bitsbadgetier" => (
        UserNoticeEvent::BitsBadgeTier {
          tier: message
            .tag(Tag::MsgParamThreshold)
            .and_then(|v| v.parse().ok())?,
        },
        false,
      ),
      _ => (UserNoticeEvent::__non_exhaustive, true),
    };

    let sender = if !is_anon {
      Some(User {
        id: message.tag(Tag::UserId)?,
        login: message.tag(Tag::Login)?,
        name: message.tag(Tag::DisplayName)?,
      })
    } else {
      None
    };

    Some(UserNotice {
      channel: message.channel()?,
      channel_id: message.tag(Tag::RoomId)?,
      sender,
      text: message.text(),
      system_message: message.tag(Tag::SystemMsg)?,
      event,
      event_id,
      badges: parse_badges(message.tag(Tag::Badges)?, message.tag(Tag::BadgeInfo)?),
      emotes: message
        .tag(Tag::Emotes)
        .map(parse_emotes)
        .unwrap_or_default(),
      color: message.tag(Tag::Color),
      message_id: message.tag(Tag::Id)?,
      timestamp: message.tag(Tag::TmiSentTs).and_then(parse_timestamp)?,
    })
  }
}

impl<'src> From<UserNotice<'src>> for super::Message<'src> {
  fn from(msg: UserNotice<'src>) -> Self {
    super::Message::UserNotice(msg)
  }
}
