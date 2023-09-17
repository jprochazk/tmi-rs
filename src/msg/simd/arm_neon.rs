//! This implementation works exactly like the `x86_sse` one,
//! but the method of finding the occurence and position of characters
//! is different because of the lack of a `movemask` equivalent in NEON.
//!
//! The method used is explained in the following article (the link goes to a table of operations):
//! https://community.arm.com/arm-community-blogs/b/infrastructure-solutions-blog/posts/porting-x86-vector-bitmask-optimizations-to-arm-neon#:~:text=Consider%20the%C2%A0result%20in%20both%20cases%20as%20the%20result%20of%20PMOVMSKB%20or%20shrn
//!
//! Archived link: https://web.archive.org/web/20230603011837/https://community.arm.com/arm-community-blogs/b/infrastructure-solutions-blog/posts/porting-x86-vector-bitmask-optimizations-to-arm-neon

use crate::msg::{leak, ParsedTags, Tags, Whitelist};

use core::arch::aarch64 as simd;
use core::mem;
use simd::uint8x16_t;
use std::ops::Add;

/// We don't have a SIMD implementation of `parse_prefix` in NEON,
/// because it was not faster. Instead just re-export the scalar impl.
pub use crate::msg::scalar::parse_prefix;

/// Parse IRC message tags:
///
/// `@key=value;other=etc `
///
/// Tags consist of semicolon-separated key-value pairs.
/// The tag list is terminated by a ` ` character.
#[inline(always)]
pub fn parse_tags<'src, const IC: usize, F>(
  remainder: &'src str,
  whitelist: &Whitelist<IC, F>,
) -> (Option<ParsedTags<'static>>, &'src str)
where
  F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
{
  // This code is identical to the `x86_sse` version.
  // It should not be duplicated, but seeing as there are only two SIMD implementations,
  // I believe it is simpler to just copy the implementation, at least for now.

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

    (Some(tags), remainder)
  } else {
    (None, remainder)
  }
}

#[inline(always)]
fn chunk16_test<T, F>(s: &str, test: F) -> Option<T>
where
  T: Add<usize, Output = T>,
  F: Fn(uint8x16_t) -> Option<T>,
{
  // This code is almost the same as `x86_sse`.
  // The only difference is that NEON does not have alignment requirements
  // for 8x16 vectors, so we use `vld1q_u8` in both the 16-byte chunk loop,
  // and for any trailing characters.

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
    buf[..bytes.len() - i].copy_from_slice(&bytes[i..]); // memcpy

    let data = unsafe { simd::vld1q_u8(buf.as_ptr() as *const _) };
    if let Some(pos) = test(data) {
      return Some(pos + i);
    }
  }

  None
}

/// Find the first `=` character in `s`.
///
/// The implementation splits `s` into 16-byte chunks, loading each chunk into a single 8x16 vector.
///
/// The resulting 8x16 vectors are compared against the pre-filled vector of a single character using `vceqq_u8`.
/// Next, the 8x16 is reinterpreted as 16x8, to which we apply `vshrn_n_u16`.
///
/// `vshrn_n_u16` performs a "vector shift right by constant and narrow".
/// The way I understand it is that for every 16-bit element in the vector,
/// it "snips off" the 4 most significant bits + 4 least significant bits:
///
/// ```text,ignore
/// # for a single element:
/// 1111111100000000 -> shift right by 4
/// 0000111111110000 -> narrow to u8
///         11110000
/// ```
///
/// If we count the number of bits in the vector before the first bit set to `1`,
/// then divide that number by `4`, we get the same result as a `movemask + ctz` would give us.
///
/// So the last step is to reinterpret the resulting 8x8 vector as a single 64-bit integer,
/// which is our mask.
/// Just like before, we can check for the presence of the "needle" by comparing the mask
/// against `0`.
/// To obtain the position of the charater, divide its trailing zeros by 4.
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
  /// Compare `a` to `b`, and produce a mask similar to the one produced by `movemask`,
  /// but with 4 bits set per character instead of 1.
  #[inline(always)]
  unsafe fn eq(a: uint8x16_t, b: uint8x16_t) -> Self {
    let mask = simd::vreinterpretq_u16_u8(simd::vceqq_u8(a, b));
    let res = simd::vshrn_n_u16(mask, 4); // the magic sauce
    let matches = simd::vget_lane_u64(simd::vreinterpret_u64_u8(res), 0);
    Mask(matches)
  }

  #[inline(always)]
  fn has_match(&self) -> bool {
    // We have a match if the mask is not empty.
    self.0 != 0
  }

  #[inline(always)]
  fn first_match_index(&self) -> usize {
    // There are 4 bits per character, so divide the trailing zeros by 4 (shift right by 2).
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
