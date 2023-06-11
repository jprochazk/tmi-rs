use crate::{leak, ParsedTags, Prefix, Tags, Whitelist};

use core::arch::x86_64 as simd;
use core::mem;
use simd::__m128i;
use std::ops::Add;

pub fn parse_tags<'src, const IC: usize, F>(
  remainder: &'src str,
  whitelist: &Whitelist<IC, F>,
) -> (Option<ParsedTags<'static>>, &'src str)
where
  F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
{
  if let Some(remainder) = remainder.strip_prefix('@') {
    let mut tags = Tags::with_capacity(IC);

    let mut remainder = remainder;
    while !remainder.is_empty() {
      let Some(key_end) = find_equals(remainder) else {
        break;
      };
      let value_start = key_end + 1;
      let value_end = find_semi_or_space(unsafe { remainder.get_unchecked(value_start..) })
        .map(|v| value_start + v);

      match value_end {
        Some(Found::Semi(value_end)) => {
          let key = unsafe { leak(remainder.get_unchecked(..key_end)) };
          let value = unsafe { leak(remainder.get_unchecked(value_start..value_end)) };
          whitelist.maybe_insert(&mut tags, key, value);
          remainder = unsafe { remainder.get_unchecked(value_end + 1..) };
          continue;
        }
        Some(Found::Space(value_end)) => {
          let key = unsafe { leak(remainder.get_unchecked(..key_end)) };
          let value = unsafe { leak(remainder.get_unchecked(value_start..value_end)) };
          whitelist.maybe_insert(&mut tags, key, value);
          remainder = unsafe { remainder.get_unchecked(value_end + 1..) };
          break;
        }
        None => {
          let key = unsafe { leak(remainder.get_unchecked(..key_end)) };
          let value = unsafe { leak(remainder.get_unchecked(value_start..)) };
          whitelist.maybe_insert(&mut tags, key, value);
          remainder = unsafe { remainder.get_unchecked(remainder.len()..) };
          break;
        }
      }
    }

    (Some(tags.into_boxed_slice()), remainder)
  } else {
    (None, remainder)
  }
}

pub fn parse_prefix(remainder: &str) -> (Option<Prefix<'static>>, &str) {
  if let Some(remainder) = remainder.strip_prefix(':') {
    let bytes: &[i8] = unsafe { mem::transmute(remainder.as_bytes()) };
    const SPACE: __m128i = unsafe { mem::transmute([b' ' as i8; 16]) };
    const AT: __m128i = unsafe { mem::transmute([b'@' as i8; 16]) };
    const BANG: __m128i = unsafe { mem::transmute([b'!' as i8; 16]) };

    let mut at = usize::MAX;
    let mut bang = usize::MAX;

    macro_rules! parse_chunk {
      ($i:ident, $data:ident) => {
        let end_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8($data, SPACE)) };
        let at_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8($data, AT)) };
        let bang_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8($data, BANG)) };

        if at_mask != 0 {
          at = $i + at_mask.trailing_zeros() as usize
        };
        if bang_mask != 0 {
          bang = $i + bang_mask.trailing_zeros() as usize
        };

        if end_mask != 0 {
          let end = $i + end_mask.trailing_zeros() as usize;

          let (prefix, remainder) = unsafe {
            (
              remainder.get_unchecked(..end),
              remainder.get_unchecked(end + 1..),
            )
          };

          let prefix = match (bang, at) {
            (usize::MAX, usize::MAX) => Prefix {
              nick: None,
              user: None,
              host: unsafe { leak(prefix) },
            },
            (usize::MAX, _) => Prefix {
              nick: Some(unsafe { leak(prefix.get_unchecked(..at)) }),
              user: None,
              host: unsafe { leak(prefix.get_unchecked(at + 1..)) },
            },
            // nick!host -> invalid
            (_, usize::MAX) => return (None, remainder),
            (bang, at) => Prefix {
              nick: Some(unsafe { leak(prefix.get_unchecked(..bang)) }),
              user: Some(unsafe { leak(prefix.get_unchecked(bang + 1..at)) }),
              host: unsafe { leak(prefix.get_unchecked(at + 1..)) },
            },
          };

          return (Some(prefix), remainder);
        }
      };
    }

    let mut i = 0usize;
    while i + 16 <= bytes.len() {
      let data = unsafe { simd::_mm_loadu_si128(bytes.as_ptr().add(i) as *const _) };

      parse_chunk!(i, data);

      i += 16;
    }
    if i < bytes.len() {
      #[repr(align(16))]
      struct Data([i8; 16]);
      let mut buf = Data([0; 16]);
      buf.0[..bytes.len() - i].copy_from_slice(&bytes[i..]);
      let data = unsafe { simd::_mm_load_si128(buf.0.as_ptr() as *const _) };

      parse_chunk!(i, data);
    }
  }

  (None, remainder)
}

#[inline]
fn chunk16_test<T, F>(s: &str, test: F) -> Option<T>
where
  T: Add<usize, Output = T>,
  F: Fn(__m128i) -> Option<T>,
{
  let bytes: &[i8] = unsafe { mem::transmute(s.as_bytes()) };

  let mut i = 0usize;
  while i + 16 <= bytes.len() {
    let data = unsafe { simd::_mm_loadu_si128(bytes.as_ptr().add(i) as *const _) };
    if let Some(pos) = test(data) {
      return Some(pos + i);
    };
    i += 16;
  }
  if i < bytes.len() {
    #[repr(align(16))]
    struct Data([i8; 16]);
    let mut buf = Data([0; 16]);
    buf.0[..bytes.len() - i].copy_from_slice(&bytes[i..]);
    let data = unsafe { simd::_mm_load_si128(buf.0.as_ptr() as *const _) };
    if let Some(pos) = test(data) {
      return Some(pos + i);
    }
  }

  None
}

#[inline(always)]
fn find_equals(s: &str) -> Option<usize> {
  #[inline(always)]
  fn test(data: __m128i) -> Option<usize> {
    const EQUALS: __m128i = unsafe { mem::transmute([b'=' as i8; 16]) };
    let mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8(data, EQUALS)) };
    if mask != 0 {
      Some(mask.trailing_zeros() as usize)
    } else {
      None
    }
  }

  chunk16_test(s, test)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Found {
  Semi(usize),
  Space(usize),
}

impl Add<usize> for Found {
  type Output = Self;

  fn add(self, rhs: usize) -> Self::Output {
    match self {
      Found::Semi(v) => Found::Semi(v + rhs),
      Found::Space(v) => Found::Space(v + rhs),
    }
  }
}

impl Add<Found> for usize {
  type Output = Found;

  fn add(self, rhs: Found) -> Self::Output {
    match rhs {
      Found::Semi(v) => Found::Semi(self + v),
      Found::Space(v) => Found::Space(self + v),
    }
  }
}

#[inline(always)]
fn find_semi_or_space(s: &str) -> Option<Found> {
  #[inline(always)]
  fn test(data: __m128i) -> Option<Found> {
    const SEMI: __m128i = unsafe { mem::transmute([b';' as i8; 16]) };
    const SPACE: __m128i = unsafe { mem::transmute([b' ' as i8; 16]) };

    let semi_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8(data, SEMI)) };
    let space_mask = unsafe { simd::_mm_movemask_epi8(simd::_mm_cmpeq_epi8(data, SPACE)) };

    match (semi_mask != 0, space_mask != 0) {
      (true, true) => {
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

  chunk16_test(s, test)
}

#[cfg(test)]
mod tests {
  use crate::whitelist_insert_all;

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

  #[test]
  fn tags() {
    macro_rules! make {
      ($($key:ident: $value:expr),* $(,)?) => (
        [
          $(($crate::Tag::$key, $value)),*
        ].into_iter().collect::<Tags>()
      );
    }

    let cases = [
      ("", (None, "")),
      ("mod=0;id=1000", (None, "mod=0;id=1000")),
      ("@mod=0;id=1000", (Some(make! {Mod: "0", Id: "1000",}), "")),
      ("@mod=0;id=1000 ", (Some(make! {Mod: "0", Id: "1000",}), "")),
      (
        "@mod=0;id=1000 :asdf",
        (Some(make! {Mod: "0", Id: "1000",}), ":asdf"),
      ),
    ];

    for (i, (string, expected)) in cases.into_iter().enumerate() {
      let result = parse_tags(string, &Whitelist::<16, _>(whitelist_insert_all));
      if result.1 != expected.1 || result.0.as_deref() != expected.0.as_deref() {
        eprintln!("[{i}] actual: {result:?}, expected: {expected:?}");
        panic!()
      }
    }
  }

  #[test]
  fn tags_whitelist() {
    macro_rules! make {
      ($($key:ident: $value:expr),* $(,)?) => (
        [
          $(($crate::Tag::$key, $value)),*
        ].into_iter().collect::<Tags>()
      );
    }

    let cases = [
      ("", (None, "")),
      ("mod=0;id=1000", (None, "mod=0;id=1000")),
      ("@mod=0;id=1000", (Some(make! {Mod: "0"}), "")),
      ("@mod=0;id=1000 ", (Some(make! {Mod: "0"}), "")),
      ("@mod=0;id=1000 :asdf", (Some(make! {Mod: "0"}), ":asdf")),
    ];

    for (i, (string, expected)) in cases.into_iter().enumerate() {
      let result = parse_tags(string, &whitelist!(Mod));
      if result.1 != expected.1 || result.0.as_deref() != expected.0.as_deref() {
        eprintln!("[{i}] actual: {result:?}, expected: {expected:?}");
        panic!()
      }
    }
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
}
