use super::*;
use crate::irc::wide::Vector as V;

pub(crate) fn parse(src: &str, pos: &mut usize) -> Option<RawTags> {
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

#[derive(Clone, Copy)]
enum State {
  Key { key_start: usize },
  Value { key_start: usize, key_end: usize },
}

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

        let m = vector_eq.first_match();
        vector_eq.clear_to(m);

        let pos = offset + m.as_index(); // pos of `=`

        *state = State::Value {
          key_start,
          key_end: pos,
        };
      }
      State::Value { key_start, key_end } => {
        if !vector_semi.has_match() {
          break;
        }

        let m = vector_semi.first_match();
        vector_semi.clear_to(m);

        let pos = offset + m.as_index(); // pos of `;`

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

// I didn't want to use runtime feature detection, or bring in a dependency for this.
//
// This implementation is ported from BurntSushi/memchr to use our vector/mask types:
// https://github.com/BurntSushi/memchr/blob/7fccf70e2a58c1fbedc9b9687c2ba0cf5992537b/src/arch/generic/memchr.rs#L143-L144
//
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
    return Some(mask.first_match().as_index());
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
    let chunk_0 = V::load_aligned(data, offset + V::SIZE * 0).eq(byte);
    let chunk_1 = V::load_aligned(data, offset + V::SIZE * 1).eq(byte);
    let chunk_2 = V::load_aligned(data, offset + V::SIZE * 2).eq(byte);
    let chunk_3 = V::load_aligned(data, offset + V::SIZE * 3).eq(byte);

    // TODO: movemask_will_have_non_zero

    let mask = chunk_0.movemask();
    if mask.has_match() {
      let pos = mask.first_match().as_index();
      return Some(offset + pos + 0 * V::SIZE);
    }

    let mask = chunk_1.movemask();
    if mask.has_match() {
      let pos = mask.first_match().as_index();
      return Some(offset + pos + 1 * V::SIZE);
    }

    let mask = chunk_2.movemask();
    if mask.has_match() {
      let pos = mask.first_match().as_index();
      return Some(offset + pos + 2 * V::SIZE);
    }

    let mask = chunk_3.movemask();
    if mask.has_match() {
      let pos = mask.first_match().as_index();
      return Some(offset + pos + 3 * V::SIZE);
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
      let pos = mask.first_match().as_index();
      return Some(offset + pos);
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
      let pos = mask.first_match().as_index();
      return Some(offset + pos);
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
}
