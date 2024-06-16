use core::arch::x86_64::{
  __m128i, _mm_cmpeq_epi8, _mm_load_si128, _mm_loadu_si128, _mm_movemask_epi8,
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

#[repr(align(16))]
struct Align16([u8; 16]);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vector128(__m128i);

impl Vector128 {
  #[inline]
  pub const fn fill(v: u8) -> Self {
    Self(unsafe { core::mem::transmute::<[u8; 16], __m128i>([v; 16]) })
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
    Self(_mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i))
  }

  /// Load 16 bytes from the given slice into a vector.
  ///
  /// - `data` must be aligned to 16 bytes.
  /// - `data[offset..].len()` must be greater than 16 bytes.
  #[inline(always)]
  unsafe fn load_aligned_16(data: &[u8], offset: usize) -> Self {
    debug_assert!(data[offset..].len() >= 16);
    debug_assert!(data.as_ptr().cast::<Align16>().is_aligned());
    Self(_mm_load_si128(data.as_ptr().add(offset) as *const __m128i))
  }

  /// Load at most 16 bytes from the given slice into a vector
  /// by loading it into an intermediate buffer on the stack.
  #[inline(always)]
  unsafe fn load_unaligned_remainder(data: &[u8], offset: usize) -> Self {
    let mut buf = Align16([0; 16]);
    buf.0[..data.len() - offset].copy_from_slice(&data[offset..]);

    Self(unsafe { _mm_load_si128(buf.0.as_ptr() as *const __m128i) })
  }

  #[inline(always)]
  pub fn find_first(self, byte: u8) -> Option<usize> {
    let mask = self.cmpeq(Self::fill(byte)).movemask();
    if mask.has_match() {
      Some(mask.first_match_index())
    } else {
      None
    }
  }

  /// Compare 16 8-bit elements in `self` against `other`, leaving a `1` in each
  #[inline(always)]
  fn cmpeq(self, other: Self) -> Self {
    unsafe { Self(_mm_cmpeq_epi8(self.0, other.0)) }
  }

  #[inline(always)]
  fn movemask(self) -> Mask16 {
    Mask16(unsafe { _mm_movemask_epi8(self.0) })
  }
}

// 1. get next 16-byte aligned offset in the data = aligned_start
// 2. use scalar method to handle values up to aligned_start
// 3. use aligned loads to find

#[derive(Clone, Copy)]
#[repr(transparent)]
struct Mask16(i32);

impl Mask16 {
  #[inline(always)]
  fn has_match(&self) -> bool {
    self.0 != 0
  }

  #[inline(always)]
  fn first_match_index(&self) -> usize {
    self.0.trailing_zeros() as usize
  }
}
