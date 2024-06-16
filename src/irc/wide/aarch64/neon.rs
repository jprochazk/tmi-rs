// The method used for emulating movemask is explained in the following article (the link goes to a table of operations):
// https://community.arm.com/arm-community-blogs/b/infrastructure-solutions-blog/posts/porting-x86-vector-bitmask-optimizations-to-arm-neon#:~:text=Consider%20the%C2%A0result%20in%20both%20cases%20as%20the%20result%20of%20PMOVMSKB%20or%20shrn
//
// Archived link: https://web.archive.org/web/20230603011837/https://community.arm.com/arm-community-blogs/b/infrastructure-solutions-blog/posts/porting-x86-vector-bitmask-optimizations-to-arm-neon
//
// For example, to find the first `=` character in `s`:
//
// The implementation splits `s` into 16-byte chunks, loading each chunk into a single 8x16 vector.
//
// The resulting 8x16 vectors are compared against the pre-filled vector of a single character using `vceqq_u8`.
// Next, the 8x16 is reinterpreted as 16x8, to which we apply `vshrn_n_u16`.
//
// `vshrn_n_u16` performs a "vector shift right by constant and narrow".
// The way I understand it is that for every 16-bit element in the vector,
// it "snips off" the 4 most significant bits + 4 least significant bits:
//
// ```text,ignore
// # for a single element:
// 1111111100000000 -> shift right by 4
// 0000111111110000 -> narrow to u8
//         11110000
// ```
//
// If we count the number of bits in the vector before the first bit set to `1`,
// then divide that number by `4`, we get the same result as a `movemask + ctz` would give us.
//
// So the last step is to reinterpret the resulting 8x8 vector as a single 64-bit integer,
// which is our mask.
// Just like before, we can check for the presence of the "needle" by comparing the mask
// against `0`.
// To obtain the position of the charater, divide its trailing zeros by 4.

use core::arch::aarch64::{
  uint8x16_t, vceqq_u8, vget_lane_u64, vld1q_u8, vreinterpret_u64_u8, vreinterpretq_u16_u8,
  vshrn_n_u16,
};

#[inline]
pub fn find(data: &[u8], mut offset: usize, byte: u8) -> Option<usize> {
  while offset < data.len() {
    let (chunk, next_offset) = Vector128::load(data, offset);
    if let Some(pos) = chunk.find_first(byte) {
      return Some(offset + pos);
    }
    offset = next_offset;
  }
  None
}

// NOTE: neon has no alignment requirements for loads,
//       but alignment is still better than no alignment.

#[repr(align(16))]
struct Align16([u8; 16]);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vector128(uint8x16_t);

impl Vector128 {
  #[inline]
  pub const fn fill(v: u8) -> Self {
    Self(unsafe { core::mem::transmute::<[u8; 16], uint8x16_t>([v; 16]) })
  }

  #[inline]
  pub fn load(data: &[u8], offset: usize) -> (Self, usize) {
    unsafe {
      if offset + 16 <= data.len() {
        let vector = Self::load_unaligned_16(data, offset);
        (vector, offset + 16)
      } else {
        let vector = Self::load_unaligned_remainder(data, offset);
        (vector, data.len())
      }
    }
  }

  /// Load 16 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 16 bytes.
  #[inline(always)]
  unsafe fn load_unaligned_16(data: &[u8], offset: usize) -> Self {
    debug_assert!(data[offset..].len() >= 16);
    Self(vld1q_u8(data.as_ptr().add(offset)))
  }

  /// Load at most 16 bytes from the given slice into a vector
  /// by loading it into an intermediate buffer on the stack.
  #[inline(always)]
  unsafe fn load_unaligned_remainder(data: &[u8], offset: usize) -> Self {
    let mut buf = Align16([0; 16]);
    buf.0[..data.len() - offset].copy_from_slice(&data[offset..]);

    Self(vld1q_u8(buf.0.as_ptr()))
  }

  #[inline(always)]
  pub fn find_first(self, byte: u8) -> Option<usize> {
    let mask = self.cmpeq(Vector128::fill(byte)).movemask();
    if mask.has_match() {
      Some(mask.first_match_index())
    } else {
      None
    }
  }

  #[inline(always)]
  fn cmpeq(self, other: Self) -> Self {
    unsafe { Self(vceqq_u8(self.0, other.0)) }
  }

  #[inline(always)]
  fn movemask(self) -> Mask16 {
    unsafe {
      let mask = vreinterpretq_u16_u8(self.0);
      let res = vshrn_n_u16(mask, 4); // the magic sauce
      let matches = vget_lane_u64(vreinterpret_u64_u8(res), 0);
      Mask16(matches)
    }
  }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct Mask16(u64);

impl Mask16 {
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
