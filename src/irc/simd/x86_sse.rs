use crate::irc::{RawPrefix, RawTags, Span, Whitelist};

use core::arch::x86_64 as simd;
use core::mem;
use simd::__m128i;
use std::ops::Add;

/// Parse IRC message tags:
///
/// `@key=value;other=etc `
///
/// Tags consist of semicolon-separated key-value pairs.
/// The tag list is terminated by a ` ` character.
#[inline(always)]
pub(crate) fn parse_tags<const IC: usize, F>(
  src: &str,
  pos: &mut usize,
  whitelist: &Whitelist<IC, F>,
) -> RawTags
where
  F: Fn(&str, &mut RawTags, Span, Span),
{
  if !src[*pos..].starts_with('@') {
    return RawTags::new();
  }

  // pre-allocate space for the tags
  // this uses a configurable default `IC`, which stands for `Initial Capacity`.
  // the library supports "whitelisting" tags, in which case we know the total
  // capacity we will ever need and can pre-allocate it.
  // in case we don't have a whitelist, then this will allocate 16 slots.
  let mut tags = RawTags::with_capacity(IC);

  let mut key_start = *pos + 1;
  while !src[key_start..].is_empty() {
    let Some(mut key_end) = find_equals(&src[key_start..]) else {
      // exit the loop if we don't find a key. this only happens if the input is malformed.
      break;
    };
    key_end += key_start;

    // `key_end` is inclusive, meaning `remainder[key_end] == '='`.
    // value starts after the `=` character.
    let value_start = key_end + 1;

    // value ends at `;` or ` ` character.
    match find_semi_or_space(&src[value_start..]) {
      // if we found a semicolon, then insert the tag into the buffer,
      // and attempt to find another tag.
      Some(Found::Semi(value_end)) => {
        let value_end = value_end + value_start;
        let key = Span::from(key_start..key_end);
        let value = Span::from(value_start..value_end);
        whitelist.maybe_insert(src, &mut tags, key, value);
        // advance to after the `;`
        key_start = value_end + 1;
        continue;
      }
      // if we found a space, then insert the tag into the buffer,
      // and break out of the loop.
      Some(Found::Space(value_end)) => {
        let value_end = value_end + value_start;
        let key = Span::from(key_start..key_end);
        let value = Span::from(value_start..value_end);
        whitelist.maybe_insert(src, &mut tags, key, value);
        // advance to after the ` `
        key_start = value_end + 1;
        break;
      }
      // we've somehow found neither. this only happens if the input is malformed.
      // we treat everything after the `=` as the value, and return an empty remainder.
      None => {
        let key = Span::from(key_start..key_end);
        let value = Span::from(value_start..src.len());
        whitelist.maybe_insert(src, &mut tags, key, value);
        key_start = src.len();
        break;
      }
    }
  }

  *pos = key_start;

  tags
}

/// This function splits `s` into 16-byte chunks, loads each chunk into a 128-bit vector,
/// and then calls `test` with the vector.
///
/// `test` returns:
/// - nothing, in which case we continue the search
/// - some position, in which case we end the search and return the position.
///
/// If we reach the end of the string without finding a position, then we return nothing as well.
#[inline]
fn chunk16_test<T, F>(s: &str, test: F) -> Option<T>
where
  T: Add<usize, Output = T>,
  F: Fn(__m128i) -> Option<T>,
{
  let bytes: &[i8] = unsafe { mem::transmute(s.as_bytes()) };

  let mut i = 0usize;
  while i + 16 <= bytes.len() {
    // load the data into the vector. this uses an unaligned load because we
    // cannot guarantee the alignment of the string bytes.
    let data = unsafe { simd::_mm_loadu_si128(bytes.as_ptr().add(i) as *const _) };
    if let Some(pos) = test(data) {
      // `test` returns a position local to its chunk, so we add the total offset
      // to get the real position.
      return Some(pos + i);
    };
    i += 16;
  }
  if i < bytes.len() {
    // we have less than 16 bytes remaining.
    // copy the remainder into a 16-byte buffer on the stack.
    // the buffer can be properly aligned to 16 bytes, so we can
    // use an aligned load.

    #[repr(align(16))] // force alignment
    struct Data([i8; 16]);

    let mut buf = Data([0; 16]);
    buf.0[..bytes.len() - i].copy_from_slice(&bytes[i..]); // memcpy

    // load the data into the vector.
    let data = unsafe { simd::_mm_load_si128(buf.0.as_ptr() as *const _) };
    if let Some(pos) = test(data) {
      // same as above, get the real position.
      return Some(pos + i);
    }
  }

  None
}

/// Find the first `=` character in `s`.
///
/// The implementation splits `s` into 16-byte chunks.
///
/// For each chunk, it compares each byte against the `=` character using `cmpeq`.
/// This produces a 8x16 vector with each 8-bit element set to:
/// - `0` if the comparison failed
/// - `255` if the comparison succeeded
///
/// `movemask` is applied to the vector, which returns a mask of the MSB of each 8-bit element.
/// The resulting mask contains:
/// - `0` if no `=` was found. In this case, we move to the next chunk.
/// - Some bits set to `1` if the character in that position was `=`.
///   In this case, we retrieve the position of the character by counting the trailing zeros,
///   and return it.
#[inline(always)]
fn find_equals(s: &str) -> Option<usize> {
  #[inline(always)]
  fn test(data: __m128i) -> Option<usize> {
    // Put `=` in each element of the vector
    const EQUALS: __m128i = unsafe { mem::transmute([b'=' as i8; 16]) };
    // compare each element in `data` against `EQUALS`
    // then use `movemask` to obtain mask which can be later used to obtain the position
    let mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8(data, EQUALS)) };

    // if the mask is not empty, return the trailing zeros
    if mask != 0 {
      Some(mask.trailing_zeros() as usize)
    } else {
      None
    }
  }

  // execute `test` for each 16-byte chunk of `s`
  chunk16_test(s, test)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Found {
  Semi(usize),
  Space(usize),
}

impl Add<usize> for Found {
  type Output = Self;

  #[inline(always)]
  fn add(self, rhs: usize) -> Self::Output {
    match self {
      Found::Semi(v) => Found::Semi(v + rhs),
      Found::Space(v) => Found::Space(v + rhs),
    }
  }
}

impl Add<Found> for usize {
  type Output = Found;

  #[inline(always)]
  fn add(self, rhs: Found) -> Self::Output {
    match rhs {
      Found::Semi(v) => Found::Semi(self + v),
      Found::Space(v) => Found::Space(self + v),
    }
  }
}

/// Find the first `;` or ` ` character in `s`.
///
/// If both are present in `s`, the one earlier one will be returned.
///
/// This works exactly like `find_equals`, but performs two comparisons at a time
/// in separate vectors, one for `;` and one for ` `.
#[inline(always)]
fn find_semi_or_space(s: &str) -> Option<Found> {
  #[inline(always)]
  fn test(data: __m128i) -> Option<Found> {
    // put `;` in each element of the vector
    const SEMI: __m128i = unsafe { mem::transmute([b';' as i8; 16]) };
    // put ` ` in each element of the vector
    const SPACE: __m128i = unsafe { mem::transmute([b' ' as i8; 16]) };

    // compare each element in `data` against `SEMI`
    // then use `movemask` to obtain mask which can be later used to obtain the position
    let semi_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8(data, SEMI)) };
    let space_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8(data, SPACE)) };

    match (semi_mask != 0, space_mask != 0) {
      (true, true) => {
        // both characters were found, return the smallest position
        let semi_tz = semi_mask.trailing_zeros() as usize;
        let space_tz = space_mask.trailing_zeros() as usize;
        if semi_tz < space_tz {
          Some(Found::Semi(semi_tz))
        } else {
          Some(Found::Space(space_tz))
        }
      }
      (true, false) => Some(Found::Semi(semi_mask.trailing_zeros() as usize)),
      (false, true) => Some(Found::Space(space_mask.trailing_zeros() as usize)),
      _ => None,
    }
  }

  // execute `test` for each 16-byte chunk of `s`
  chunk16_test(s, test)
}

/// Parse an IRC message prefix:
///
/// `:host`
/// `:nick@host`
/// `:nick!user@host`
///
/// Twitch never sends the `nick@host` form, but we still handle it.
#[inline(always)]
pub(crate) fn parse_prefix(src: &str, pos: &mut usize) -> Option<RawPrefix> {
  const SPACE: __m128i = unsafe { mem::transmute([b' ' as i8; 16]) };
  const AT: __m128i = unsafe { mem::transmute([b'@' as i8; 16]) };
  const BANG: __m128i = unsafe { mem::transmute([b'!' as i8; 16]) };

  macro_rules! parse_chunk {
    ($i:ident, $at:ident, $bang:ident, $data:ident) => {
      let end_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8($data, SPACE)) };
      let at_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8($data, AT)) };
      let bang_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8($data, BANG)) };

      if at_mask != 0 {
        $at = $i + at_mask.trailing_zeros() as usize
      };
      if bang_mask != 0 {
        $bang = $i + bang_mask.trailing_zeros() as usize
      };

      if end_mask != 0 {
        let end = $i + end_mask.trailing_zeros() as usize;

        let prefix = match ($bang, $at) {
          (usize::MAX, usize::MAX) => RawPrefix {
            nick: None,
            user: None,
            host: Span::from($i..end),
          },
          (usize::MAX, _) => RawPrefix {
            nick: Some(Span::from($i..$at)),
            user: None,
            host: Span::from($at + 1..end),
          },
          // nick!host -> invalid
          (_, usize::MAX) => return None,
          (bang, at) => RawPrefix {
            nick: Some(Span::from($i..bang)),
            user: Some(Span::from(bang + 1..at)),
            host: Span::from(at + 1..end),
          },
        };

        *pos = end + 1;

        return Some(prefix);
      }
    };
  }

  if !src[*pos..].starts_with(':') {
    return None;
  }

  let start = *pos + 1;
  let bytes: &[i8] = unsafe { mem::transmute(src.as_bytes()) };

  let mut at = usize::MAX;
  let mut bang = usize::MAX;

  let mut i = start;
  while i + 16 <= bytes.len() {
    let data = unsafe { simd::_mm_loadu_si128(bytes.as_ptr().add(i) as *const _) };

    parse_chunk!(i, at, bang, data);

    i += 16;
  }
  if i < bytes.len() {
    #[repr(align(16))]
    struct Data([i8; 16]);
    let mut buf = Data([0; 16]);
    buf.0[..bytes.len() - i].copy_from_slice(&bytes[i..]);
    let data = unsafe { simd::_mm_load_si128(buf.0.as_ptr() as *const _) };

    parse_chunk!(i, at, bang, data);
  }

  None
}

#[cfg(test)]
mod tests {
  use crate::irc::whitelist_insert_all;

  use super::*;

  #[test]
  fn equals() {
    let cases = [
      ("", None),
      ("asdf=", Some(4)),
      ("=asdf", Some(0)),
      ("as=df", Some(2)),
    ];

    for (string, expected) in cases {
      assert_eq!(find_equals(string), expected);
    }
  }

  #[test]
  fn semi_or_space() {
    use Found::*;

    let cases = [
      ("", None),
      (" ", Some(Space(0))),
      (";", Some(Semi(0))),
      (" ;", Some(Space(0))),
      ("; ", Some(Semi(0))),
      ("____________________; ", Some(Semi(20))),
      ("____________________ ;", Some(Space(20))),
    ];

    for (string, expected) in cases {
      assert_eq!(find_semi_or_space(string), expected);
    }
  }

  macro_rules! make {
    ($($key:ident: $value:expr),* $(,)?) => (
      [
        $(($crate::Tag::$key, $value)),*
      ].into_iter().collect::<Vec<_>>()
    );
  }

  #[test]
  fn tags() {
    let cases = [
      ("", (vec![], "")),
      ("mod=0;id=1000", (vec![], "mod=0;id=1000")),
      ("@mod=0;id=1000", (make! {Mod: "0", Id: "1000",}, "")),
      ("@mod=0;id=1000 ", (make! {Mod: "0", Id: "1000",}, "")),
      (
        "@mod=0;id=1000 :asdf",
        (make! {Mod: "0", Id: "1000",}, ":asdf"),
      ),
    ];

    for (src, (expected_tags, expected_remainder)) in cases.into_iter() {
      let mut pos = 0;
      let actual_tags: Vec<_> =
        parse_tags(src, &mut pos, &Whitelist::<16, _>(whitelist_insert_all))
          .into_iter()
          .map(|tag| tag.get(src))
          .collect();
      assert_eq!(actual_tags, expected_tags);
      assert_eq!(&src[pos..], expected_remainder);
    }
  }

  #[test]
  fn tags_whitelist() {
    let cases = [
      ("", (vec![], "")),
      ("mod=0;id=1000", (vec![], "mod=0;id=1000")),
      ("@mod=0;id=1000", (make! {Mod: "0"}, "")),
      ("@mod=0;id=1000 ", (make! {Mod: "0"}, "")),
      ("@mod=0;id=1000 :asdf", (make! {Mod: "0"}, ":asdf")),
    ];

    for (src, (expected_tags, expected_remainder)) in cases.into_iter() {
      let mut pos = 0;
      let actual_tags: Vec<_> = parse_tags(src, &mut pos, &whitelist!(Mod))
        .into_iter()
        .map(|tag| tag.get(src))
        .collect();
      assert_eq!(actual_tags, expected_tags);
      assert_eq!(&src[pos..], expected_remainder)
    }
  }

  #[test]
  fn prefix() {
    let data = ":nick!user@host <rest>";
    let mut pos = 0;
    let prefix = parse_prefix(data, &mut pos).unwrap();
    assert_eq!(prefix.nick.unwrap().get(data), "nick");
    assert_eq!(prefix.user.unwrap().get(data), "user");
    assert_eq!(prefix.host.get(data), "host");
    assert_eq!(&data[pos..], "<rest>");

    let data = ":nick@host <rest>";
    let mut pos = 0;
    let prefix = parse_prefix(data, &mut pos).unwrap();
    assert_eq!(prefix.nick.unwrap().get(data), "nick");
    assert!(prefix.user.is_none());
    assert_eq!(prefix.host.get(data), "host");
    assert_eq!(&data[pos..], "<rest>");

    let data = ":host <rest>";
    let mut pos = 0;
    let prefix = parse_prefix(data, &mut pos).unwrap();
    assert!(prefix.nick.is_none());
    assert!(prefix.user.is_none());
    assert_eq!(prefix.host.get(data), "host");
    assert_eq!(&data[pos..], "<rest>");
  }

  #[test]
  fn test_parse_data_0() {
    crate::IrcMessageRef::parse(r"@badge-info=;badges=premium/1;color=#000000;display-name=Vicarun;emotes=;flags=;id=a0414f65-b471-46be-b6cc-f8d7cd0aa62c;login=vicarun;mod=0;msg-id=resub;msg-param-cumulative-months=20;msg-param-months=0;msg-param-multimonth-duration=1;msg-param-multimonth-tenure=0;msg-param-should-share-streak=0;msg-param-sub-plan-name=Channel\sSubscription\s(forsenlol);msg-param-sub-plan=Prime;msg-param-was-gifted=false;room-id=22484632;subscriber=1;system-msg=Vicarun\ssubscribed\swith\sPrime.\sThey've\ssubscribed\sfor\s20\smonths!;tmi-sent-ts=1685664553875;user-id=691811336;user-type= :tmi.twitch.tv USERNOTICE #forsen").unwrap();
  }
}
