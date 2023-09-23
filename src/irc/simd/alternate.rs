// NOTE: this code has a lot of bitrot, but it's left here as a proof of concept

use core::arch::x86_64;
use std::arch::x86_64::{
  _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm512_cmpeq_epi8_mask,
  _mm512_loadu_si512, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8,
};
use std::mem::transmute;

use crate::msg::{leak, ParsedTags, Tags, Whitelist};

const LANE_SIZE: usize = 64;
type Vector = x86_64::__m512i;

const fn fill(e: u8) -> Vector {
  unsafe { transmute([e; LANE_SIZE]) }
}

pub use crate::msg::scalar::parse_prefix;

#[inline(always)]
pub(crate) fn parse_tags<'src, const IC: usize, F>(
  remainder: &'src str,
  whitelist: &Whitelist<IC, F>,
) -> (Option<ParsedTags<'static>>, &'src str)
where
  F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
{
  let Some(remainder) = remainder.strip_prefix('@') else {
    return (None, remainder);
  };

  let mut tags = Tags::with_capacity(16);
  let mut key: &str = "";
  let mut in_value = false;
  let mut offset = 0;
  let mut chunks = remainder.as_bytes().chunks_exact(LANE_SIZE);
  for (chunk_index, chunk) in chunks.by_ref().enumerate() {
    let base_index = chunk_index * LANE_SIZE;
    let vector = unsafe { _mm512_loadu_si512(chunk.as_ptr() as *const _) };

    const EQ: Vector = fill(b'=');
    const SEMI: Vector = fill(b';');
    const SPACE: Vector = fill(b' ');

    let mut mask = unsafe { _mm512_cmpeq_epi8_mask(vector, EQ) }
      | unsafe { _mm512_cmpeq_epi8_mask(vector, SEMI) }
      | unsafe { _mm512_cmpeq_epi8_mask(vector, SPACE) };
    while mask != 0 {
      let next_char_pos = mask.trailing_zeros() as usize;
      mask &= !(1 << next_char_pos);

      let prev_offset = offset;
      let end_index = base_index + next_char_pos;
      let token = unsafe { leak(&remainder[offset..end_index]) };
      offset = end_index + 1;

      match chunk[next_char_pos] {
        b'=' if in_value => {
          offset = prev_offset;
        }
        b'=' => {
          key = token;
          in_value = true;
        }
        b';' => {
          whitelist.maybe_insert(&mut tags, key, token);
          in_value = false;
        }
        b' ' => {
          whitelist.maybe_insert(&mut tags, key, token);
          return (Some(ParsedTags::from(tags)), &remainder[offset..]);
        }
        _ => unreachable!(),
      }
    }
  }

  unreachable!()
}

/*

TAGS_BEGIN = '@'
TAGS_END = ' '
KEY_END = '='
VALUE_END = ';'



*/

#[cfg(test)]
mod tests {
  use crate::whitelist_insert_all;

  use super::*;

  #[test]
  fn tags() {
    macro_rules! make {
      ($($key:ident: $value:expr),* $(,)?) => (
        $crate::ParsedTags::from([
          $(($crate::Tag::$key, $value)),*
        ].into_iter().collect::<Tags>())
      );
    }

    let cases = [
      ("", (None, "")),
      ("mod=0;id=1000", (None, "mod=0;id=1000")),
      (
        "@mod=0;id=1000 test",
        (Some(make! {Mod: "0", Id: "1000",}), "test"),
      ),
    ];

    for (n, (input, expected)) in cases.into_iter().enumerate() {
      let actual = parse_tags(input, &Whitelist::<16, _>(whitelist_insert_all));
      if actual != expected {
        eprintln!("[{n}] actual: {actual:?}, expected: {expected:?}");
        panic!()
      }
    }
  }
}
