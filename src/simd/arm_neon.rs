use crate::{leak, ParsedTags, Tags, Whitelist};

use core::arch::aarch64 as simd;
use core::mem;
use simd::uint8x16_t;
use std::ops::Add;

pub use crate::parse_prefix;

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

#[inline]
fn chunk16_test<T, F>(s: &str, test: F) -> Option<T>
where
  T: Add<usize, Output = T>,
  F: Fn(uint8x16_t) -> Option<T>,
{
  let bytes = s.as_bytes();

  let mut i = 0usize;
  while i + 16 <= bytes.len() {
    let data = unsafe { simd::vld1q_u8(bytes.as_ptr().add(i) as *const _) };
    if let Some(pos) = test(data) {
      return Some(pos + i);
    };
    i += 16;
  }
  if i < bytes.len() {
    let mut buf = [0; 16];
    buf[..bytes.len() - i].copy_from_slice(&bytes[i..]);
    let data = unsafe { simd::vld1q_u8(buf.as_ptr() as *const _) };
    if let Some(pos) = test(data) {
      return Some(pos + i);
    }
  }

  None
}

#[inline(always)]
fn find_equals(s: &str) -> Option<usize> {
  #[inline(always)]
  fn test(data: uint8x16_t) -> Option<usize> {
    const EQUALS: uint8x16_t = unsafe { mem::transmute([b'='; 16]) };
    let mask = unsafe { Mask::eq(data, EQUALS) };
    if mask.has_match() {
      Some(mask.first_match_index())
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
  fn test(data: uint8x16_t) -> Option<Found> {
    const SEMI: uint8x16_t = unsafe { mem::transmute([b';'; 16]) };
    const SPACE: uint8x16_t = unsafe { mem::transmute([b' '; 16]) };

    let semi_mask = unsafe { Mask::eq(data, SEMI) };
    let space_mask = unsafe { Mask::eq(data, SPACE) };

    match (semi_mask.has_match(), space_mask.has_match()) {
      (true, true) => {
        let semi = semi_mask.first_match_index();
        let space = space_mask.first_match_index();
        if semi < space {
          Some(Found::Semi(semi))
        } else {
          Some(Found::Space(space))
        }
      }
      (true, false) => Some(Found::Semi(semi_mask.first_match_index())),
      (false, true) => Some(Found::Space(space_mask.first_match_index())),
      _ => None,
    }
  }

  chunk16_test(s, test)
}

struct Mask(u64);

impl Mask {
  #[inline(always)]
  unsafe fn eq(a: uint8x16_t, b: uint8x16_t) -> Self {
    let mask = simd::vreinterpretq_u16_u8(simd::vceqq_u8(a, b));
    let res = simd::vshrn_n_u16(mask, 4);
    let matches = simd::vget_lane_u64(simd::vreinterpret_u64_u8(res), 0);
    Mask(matches)
  }

  #[inline(always)]
  fn has_match(&self) -> bool {
    self.0 != 0
  }

  #[inline(always)]
  fn first_match_index(&self) -> usize {
    (self.0.trailing_zeros() >> 2) as usize
  }
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
}
