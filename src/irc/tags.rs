use std::fmt::Display;
use std::ops::Deref;

use crate::common::Span;

macro_rules! tags_def {
  (
    $tag:ident, $raw_tag:ident, $tag_mod:ident;
    $($(#[$meta:meta])* $bytes:literal; $key:literal = $name:ident),* $(,)?
  ) => {
    /// A parsed tag value.
    #[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
    #[non_exhaustive]
    pub enum $tag<'src> {
      $(
        $(#[$meta])*
        $name,
      )*
      Unknown(&'src str),
    }

    impl<'src> $tag<'src> {
      #[doc = concat!("Get the string value of the [`", stringify!($tag), "`].")]
      #[inline]
      pub fn as_str(&self) -> &'src str {
        match self {
          $(Self::$name => $key,)*
          Self::Unknown(key) => key,
        }
      }

      #[doc = concat!("Parse a [`", stringify!($tag), "`] from a string.")]
      #[inline]
      pub fn parse(src: &'src str) -> Self {
        match src.as_bytes() {
          $($bytes => Self::$name,)*
          _ => Self::Unknown(src),
        }
      }
    }

    #[derive(Clone, Copy)]
    #[non_exhaustive]
    pub(super) enum $raw_tag {
      $($name,)*
      Unknown(Span),
    }

    impl $raw_tag {
      #[inline]
      fn get<'src>(&self, src: &'src str) -> $tag<'src> {
        match self {
          $(Self::$name => $tag::$name,)*
          Self::Unknown(span) => $tag::Unknown(&src[*span]),
        }
      }

      #[inline(never)]
      pub(super) fn parse(src: &str, span: Span) -> Self {
        match src[span].as_bytes() {
          $($bytes => Self::$name,)*
          _ => Self::Unknown(span),
        }
      }
    }

    #[allow(non_upper_case_globals)]
    pub(super) mod $tag_mod {
      $(pub const $name: &'static [u8] = $bytes;)*
    }
  }
}

impl<'src> From<&'src str> for Tag<'src> {
  fn from(value: &'src str) -> Self {
    Tag::parse(value)
  }
}

tags_def! {
  Tag, RawTag, tags;
  b"msg-id"; "msg-id" = MsgId,
  b"badges"; "badges" = Badges,
  b"badge-info"; "badge-info" = BadgeInfo,
  b"display-name"; "display-name" = DisplayName,
  b"emote-only"; "emote-only" = EmoteOnly,
  b"emotes"; "emotes" = Emotes,
  b"flags"; "flags" = Flags,
  b"id"; "id" = Id,
  b"mod"; "mod" = Mod,
  b"room-id"; "room-id" = RoomId,
  b"subscriber"; "subscriber" = Subscriber,
  b"tmi-sent-ts"; "tmi-sent-ts" = TmiSentTs,
  b"turbo"; "turbo" = Turbo,
  b"user-id"; "user-id" = UserId,
  b"user-type"; "user-type" = UserType,
  b"client-nonce"; "client-nonce" = ClientNonce,
  b"first-msg"; "first-msg" = FirstMsg,

  b"reply-parent-display-name"; "reply-parent-display-name" = ReplyParentDisplayName,

  b"reply-parent-msg-body"; "reply-parent-msg-body" = ReplyParentMsgBody,

  /// ID of the message the user replied to.
  ///
  /// This is different from `reply-thread-parent-msg-id` as it identifies the specific message
  /// the user replied to, not the thread.
  b"reply-parent-msg-id"; "reply-parent-msg-id" = ReplyParentMsgId,

  b"reply-parent-user-id"; "reply-parent-user-id" = ReplyParentUserId,

  b"reply-parent-user-login"; "reply-parent-user-login" = ReplyParentUserLogin,

  /// Root message ID of the thread the user replied to.
  ///
  /// This never changes for a given thread, so it can be used to identify the thread.
  b"reply-thread-parent-msg-id"; "reply-thread-parent-msg-id" = ReplyThreadParentMsgId,

  /// Login of the user who posted the root message in the thread the user replied to.
  b"reply-thread-parent-user-login"; "reply-thread-parent-user-login" = ReplyThreadParentUserLogin,

  b"followers-only"; "followers-only" = FollowersOnly,
  b"r9k"; "r9k" = R9K,
  b"rituals"; "rituals" = Rituals,
  b"slow"; "slow" = Slow,
  b"subs-only"; "subs-only" = SubsOnly,
  b"msg-param-cumulative-months"; "msg-param-cumulative-months" = MsgParamCumulativeMonths,
  b"msg-param-displayName"; "msg-param-displayName" = MsgParamDisplayName,
  b"msg-param-login"; "msg-param-login" = MsgParamLogin,
  b"msg-param-months"; "msg-param-months" = MsgParamMonths,
  b"msg-param-promo-gift-total"; "msg-param-promo-gift-total" = MsgParamPromoGiftTotal,
  b"msg-param-promo-name"; "msg-param-promo-name" = MsgParamPromoName,
  b"msg-param-recipient-display-name"; "msg-param-recipient-display-name" = MsgParamRecipientDisplayName,
  b"msg-param-recipient-id"; "msg-param-recipient-id" = MsgParamRecipientId,
  b"msg-param-recipient-user-name"; "msg-param-recipient-user-name" = MsgParamRecipientUserName,
  b"msg-param-sender-login"; "msg-param-sender-login" = MsgParamSenderLogin,
  b"msg-param-sender-name"; "msg-param-sender-name" = MsgParamSenderName,
  b"msg-param-should-share-streak"; "msg-param-should-share-streak" = MsgParamShouldShareStreak,
  b"msg-param-streak-months"; "msg-param-streak-months" = MsgParamStreakMonths,
  b"msg-param-sub-plan"; "msg-param-sub-plan" = MsgParamSubPlan,
  b"msg-param-sub-plan-name"; "msg-param-sub-plan-name" = MsgParamSubPlanName,
  b"msg-param-viewerCount"; "msg-param-viewerCount" = MsgParamViewerCount,
  b"msg-param-ritual-name"; "msg-param-ritual-name" = MsgParamRitualName,
  b"msg-param-threshold"; "msg-param-threshold" = MsgParamThreshold,
  b"msg-param-gift-months"; "msg-param-gift-months" = MsgParamGiftMonths,
  b"msg-param-color"; "msg-param-color" = MsgParamColor,
  b"login"; "login" = Login,
  b"bits"; "bits" = Bits,
  b"system-msg"; "system-msg" = SystemMsg,
  b"emote-sets"; "emote-sets" = EmoteSets,
  b"thread-id"; "thread-id" = ThreadId,
  b"message-id"; "message-id" = MessageId,
  b"returning-chatter"; "returning-chatter" = ReturningChatter,
  b"color"; "color" = Color,
  b"vip"; "vip" = Vip,
  b"target-user-id"; "target-user-id" = TargetUserId,
  b"target-msg-id"; "target-msg-id" = TargetMsgId,
  b"ban-duration"; "ban-duration" = BanDuration,
  b"msg-param-multimonth-duration"; "msg-param-multimonth-duration" = MsgParamMultimonthDuration,
  b"msg-param-was-gifted"; "msg-param-was-gifted" = MsgParamWasGifted,
  b"msg-param-multimonth-tenure"; "msg-param-multimonth-tenure" = MsgParamMultimonthTenure,
  b"sent-ts"; "sent-ts" = SentTs,
  b"msg-param-origin-id"; "msg-param-origin-id" = MsgParamOriginId,
  b"msg-param-fun-string"; "msg-param-fun-string" = MsgParamFunString,
  b"msg-param-sender-count"; "msg-param-sender-count" = MsgParamSenderCount,
  b"msg-param-profileImageURL"; "msg-param-profileImageURL" = MsgParamProfileImageUrl,
  b"msg-param-mass-gift-count"; "msg-param-mass-gift-count" = MsgParamMassGiftCount,
  b"msg-param-gift-month-being-redeemed"; "msg-param-gift-month-being-redeemed" = MsgParamGiftMonthBeingRedeemed,
  b"msg-param-anon-gift"; "msg-param-anon-gift" = MsgParamAnonGift,
  b"custom-reward-id"; "custom-reward-id" = CustomRewardId,

  /// The value of the Hype Chat sent by the user.
  b"pinned-chat-paid-amount"; "pinned-chat-paid-amount" = PinnedChatPaidAmount,

  /// The value of the Hype Chat sent by the user. This seems to always be the same as `pinned-chat-paid-amount`.
  b"pinned-chat-paid-canonical-amount"; "pinned-chat-paid-amount" = PinnedChatPaidCanonicalAmount,

  /// The ISO 4217 alphabetic currency code the user has sent the Hype Chat in.
  b"pinned-chat-paid-currency"; "pinned-chat-paid-currency" = PinnedChatPaidCurrency,

  /// Indicates how many decimal points this currency represents partial amounts in.
  b"pinned-chat-paid-exponent"; "pinned-chat-paid-exponent" = PinnedChatPaidExponent,

  /// The level of the Hype Chat, in English.
  ///
  /// Possible values are capitalized words from `ONE` to `TEN`: ONE TWO THREE FOUR FIVE SIX SEVEN EIGHT NINE TEN
  b"pinned-chat-paid-level"; "pinned-chat-paid-level" = PinnedChatPaidLevel,

  /// A Boolean value that determines if the message sent with the Hype Chat was filled in by the system.
  ///
  /// If `true` (1), the user entered no message and the body message was automatically filled in by the system.
  /// If `false` (0), the user provided their own message to send with the Hype Chat.
  b"pinned-chat-paid-is-system-message"; "pinned-chat-paid-is-system-message" = PinnedChatPaidIsSystemMessage,
}

impl<'src> Display for Tag<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Default, Clone)]
pub(super) struct RawTags(pub(crate) Vec<TagPair>);

impl Deref for RawTags {
  type Target = Vec<TagPair>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl IntoIterator for RawTags {
  type Item = TagPair;

  type IntoIter = std::vec::IntoIter<TagPair>;

  fn into_iter(self) -> Self::IntoIter {
    self.0.into_iter()
  }
}

#[derive(Default, Clone, Copy)]
pub(super) struct TagPair {
  // key=value
  // ^
  key_start: u32,
  // key=value
  //    ^
  key_end: u16,

  // key=value
  //          ^
  value_end: u16,
}

impl TagPair {
  // key=value
  // ^  ^
  #[inline]
  pub fn key(&self) -> Span {
    let start = self.key_start;
    let end = start + self.key_end as u32;
    Span { start, end }
  }

  // key=value
  //     ^    ^
  #[inline]
  pub fn value(&self) -> Span {
    let start = self.key_start + self.key_end as u32 + 1;
    let end = start + self.value_end as u32;
    Span { start, end }
  }

  #[inline]
  pub fn get<'a>(&self, src: &'a str) -> (&'a str, &'a str) {
    (&src[self.key()], &src[self.value()])
  }
}

struct Array<const CAPACITY: usize, T> {
  data: [core::mem::MaybeUninit<T>; CAPACITY],
  len: usize,
}

impl<const CAPACITY: usize, T: Clone + Copy + Default> Array<CAPACITY, T> {
  fn new() -> Self {
    unsafe {
      let uninit_array = core::mem::MaybeUninit::<[T; CAPACITY]>::uninit();
      let array_of_uninit = uninit_array
        .as_ptr()
        .cast::<[core::mem::MaybeUninit<T>; CAPACITY]>()
        .read();

      Self {
        data: array_of_uninit,
        len: 0,
      }
    }
  }

  fn push(&mut self, value: T) {
    self.data[self.len].write(value);
    self.len += 1;
  }

  fn to_vec(&self) -> Vec<T> {
    let init = &self.data[..self.len];
    let init = unsafe { core::mem::transmute::<&[core::mem::MaybeUninit<T>], &[T]>(init) };
    init.to_vec()
  }
}

use super::wide;

#[derive(Clone, Copy)]
enum State {
  Key { key_start: usize },
  Value { key_start: usize, key_end: usize },
}

type V = wide::Vector;

#[inline(always)]
fn parse_chunk(offset: usize, chunk: V, state: &mut State, tags: &mut Array<128, TagPair>) {
  let mut vector_eq = chunk.eq(b'=').movemask();
  let mut vector_semi = chunk.eq(b';').movemask();

  loop {
    match *state {
      State::Key { key_start } => {
        if !vector_eq.has_match() {
          break;
        }

        let idx = vector_eq.first_match();
        vector_eq.clear_to(idx);

        let pos = offset + idx; // pos of `=`

        *state = State::Value {
          key_start,
          key_end: pos,
        };
      }
      State::Value { key_start, key_end } => {
        if !vector_semi.has_match() {
          break;
        }

        let idx = vector_semi.first_match();
        vector_semi.clear_to(idx);

        let pos = offset + idx; // pos of `;`

        *state = State::Key { key_start: pos + 1 };

        tags.push(TagPair {
          // relative to original `src`
          key_start: key_start as u32 + 1,
          key_end: (key_end - key_start) as u16,
          // starts after `=`
          value_end: (pos - (key_end + 1)) as u16,
        });
      }
    }
  }
}

pub(super) fn parse(src: &str, pos: &mut usize) -> Option<RawTags> {
  let src = src[*pos..].strip_prefix('@')?.as_bytes();

  // 1. scan for ASCII space to find tags end
  let end = find_first(src, b' ')?;
  *pos += end + 2; // skip '@' + space

  let remainder = &src[..end];
  let mut tags = Array::<128, TagPair>::new();
  let mut offset = 0;

  let mut state = State::Key { key_start: 0 };
  while offset + V::SIZE < remainder.len() {
    let chunk = V::load_unaligned(remainder, offset);
    parse_chunk(offset, chunk, &mut state, &mut tags);
    offset += V::SIZE;
  }

  if remainder.len() - offset > 0 {
    let chunk = V::load_unaligned_remainder(remainder, offset);
    parse_chunk(offset, chunk, &mut state, &mut tags);

    if let State::Value { key_start, key_end } = state {
      // value contains whatever is left after key_end

      let pos = remainder.len(); // pos of `;`

      tags.push(TagPair {
        // relative to original `src`
        key_start: key_start as u32 + 1,
        key_end: (key_end - key_start) as u16,
        // starts after `=`
        value_end: (pos - (key_end + 1)) as u16,
      });
    }
  }

  Some(RawTags(tags.to_vec()))
}

// This implementation is ported from BurntSushi/memchr:
// https://github.com/BurntSushi/memchr/blob/7fccf70e2a58c1fbedc9b9687c2ba0cf5992537b/src/arch/generic/memchr.rs#L143-L144
// The original implementation is licensed under the MIT license.
#[allow(clippy::erasing_op, clippy::identity_op, clippy::needless_range_loop)]
#[inline]
fn find_first(data: &[u8], byte: u8) -> Option<usize> {
  // 1. scalar fallback for small data
  if data.len() < V::SIZE {
    for i in 0..data.len() {
      if data[i] == byte {
        return Some(i);
      }
    }

    return None;
  }

  // 2. read the first chunk unaligned, because we are now
  //    guaranteed to have more than vector-size bytes
  let chunk = V::load_unaligned(data, 0);
  let mask = chunk.eq(byte).movemask();
  if mask.has_match() {
    return Some(mask.first_match());
  }

  // 3. read the rest of the data in vector-size aligned chunks
  const UNROLLED_BYTES: usize = 4 * V::SIZE;

  // it's fine if we overlap the next vector-size chunk with
  // some part of the first chunk, because we already know
  // that there is no match in the first vector-size bytes.
  let data_addr = data.as_ptr() as usize;
  let aligned_start_addr = data_addr + V::SIZE - (data_addr % V::SIZE);
  let aligned_start_offset = aligned_start_addr - data_addr;

  let mut offset = aligned_start_offset;
  while offset + UNROLLED_BYTES < data.len() {
    // do all loads up-front to saturate the pipeline
    let eq_0 = V::load_aligned(data, offset + V::SIZE * 0).eq(byte);
    let eq_1 = V::load_aligned(data, offset + V::SIZE * 1).eq(byte);
    let eq_2 = V::load_aligned(data, offset + V::SIZE * 2).eq(byte);
    let eq_3 = V::load_aligned(data, offset + V::SIZE * 3).eq(byte);

    // TODO: movemask_will_have_non_zero

    let mask = eq_0.movemask();
    if mask.has_match() {
      return Some(offset + mask.first_match() + 0 * V::SIZE);
    }

    let mask = eq_1.movemask();
    if mask.has_match() {
      return Some(offset + mask.first_match() + 1 * V::SIZE);
    }

    let mask = eq_2.movemask();
    if mask.has_match() {
      return Some(offset + mask.first_match() + 2 * V::SIZE);
    }

    let mask = eq_3.movemask();
    if mask.has_match() {
      return Some(offset + mask.first_match() + 3 * V::SIZE);
    }

    offset += V::SIZE * 4;
  }

  // 4. we may have fewer than UNROLLED_BYTES bytes left, which may
  //    still be enough for one or more vector-size chunks.
  while offset + V::SIZE <= data.len() {
    // the data is still guaranteed to be aligned at this point.
    let chunk = V::load_aligned(data, offset);
    let mask = chunk.eq(byte).movemask();
    if mask.has_match() {
      return Some(offset + mask.first_match());
    }

    offset += V::SIZE;
  }

  // 5. we definitely have fewer than a single vector-size chunk left,
  //    so we have to read the last chunk unaligned.
  //    note that it is fine if it overlaps with the previous chunk,
  //    for the same reason why it's fine in step 3.
  if offset < data.len() {
    let offset = data.len() - V::SIZE;

    let chunk = V::load_unaligned(data, offset);
    let mask = chunk.eq(byte).movemask();
    if mask.has_match() {
      return Some(offset + mask.first_match());
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn find_first_test() {
    fn a(size: usize, needle_at: usize) -> Vec<u8> {
      let mut data = vec![b'.'; size];
      data[needle_at] = b'x';
      data
    }

    let cases: &[(&[u8], Option<usize>)] = &[
      // sub vector-size chunks
      (b"", None),      // 0 bytes
      (b"x", Some(0)),  // 1 byte
      (b".", None),     // 1 byte
      (b"xx", Some(0)), // 2 bytes
      (b"x.", Some(0)), // 2 bytes
      (b".x", Some(1)), // 2 bytes
      // vector-size chunks
      // 16 bytes
      (b"x...............", Some(0)),
      (b".x..............", Some(1)),
      (b"..............x.", Some(14)),
      (b"...............x", Some(15)),
      // uneven + above vector-size chunks
      // 17 bytes
      (b"x................", Some(0)),
      (b".x...............", Some(1)),
      (b"...............x.", Some(15)),
      (b"................x", Some(16)),
      // 31 bytes
      (b"x...............................", Some(0)),
      (b".x..............................", Some(1)),
      (b"..............................x.", Some(30)),
      (b"...............................x", Some(31)),
      // large chunks
      // 1 KiB
      (&a(1024, 0)[..], Some(0)),
      (&a(1024, 1)[..], Some(1)),
      (&a(1024, 1022)[..], Some(1022)),
      (&a(1024, 1023)[..], Some(1023)),
    ];

    for (i, case) in cases.iter().enumerate() {
      let (data, expected) = *case;
      assert_eq!(find_first(data, b'x'), expected, "case {} failed", i);
    }
  }

  #[test]
  fn roundtrip() {
    let src = "@some-key-a=some-value-a;some-key-b=some-value-b;some-key-c=some-value-c ";

    let mut pos = 0;
    let parsed = format!(
      "@{} ",
      parse(src, &mut pos)
        .unwrap()
        .into_iter()
        .map(|tag| format!("{}={}", &src[tag.key()], &src[tag.value()]))
        .collect::<Vec<_>>()
        .join(";")
    );

    assert_eq!(&src[pos..], "");
    assert_eq!(src, parsed);
  }
}
