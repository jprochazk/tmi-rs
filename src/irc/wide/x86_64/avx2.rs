use core::arch::x86_64::{
  __m256i, _mm256_cmpeq_epi8, _mm256_load_si256, _mm256_loadu_si256, _mm256_movemask_epi8,
};

#[inline]
pub fn find(data: &[u8], mut offset: usize, byte: u8) -> Option<usize> {
  while offset < data.len() {
    let (chunk, next_offset) = Vector256::load(data, offset);
    if let Some(pos) = chunk.find_first(byte) {
      return Some(offset + pos);
    }
    offset = next_offset;
  }
  None
}

#[repr(align(32))]
struct Align32([u8; 32]);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vector256(__m256i);

impl Vector256 {
  #[inline]
  pub const fn fill(v: u8) -> Self {
    Self(unsafe { core::mem::transmute::<[u8; 32], __m256i>([v; 32]) })
  }

  #[inline(always)]
  pub fn load(data: &[u8], offset: usize) -> (Self, usize) {
    unsafe {
      if offset + 32 <= data.len() {
        let vector = Self::load_unaligned_32(data, offset);
        (vector, offset + 32)
      } else {
        let vector = Self::load_unaligned_remainder(data, offset);
        (vector, data.len())
      }
    }
  }

  /// Load 32 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 32 bytes.
  #[inline(always)]
  unsafe fn load_unaligned_32(data: &[u8], offset: usize) -> Self {
    debug_assert!(data[offset..].len() >= 32);
    Self(_mm256_loadu_si256(
      data.as_ptr().add(offset) as *const __m256i
    ))
  }

  /// Load 32 bytes from the given slice into a vector.
  ///
  /// - `data` must be aligned to 32 bytes.
  /// - `data[offset..].len()` must be greater than 32 bytes.
  #[inline(always)]
  unsafe fn load_aligned_16(data: &[u8], offset: usize) -> Self {
    debug_assert!(data[offset..].len() >= 32);
    debug_assert!(data.as_ptr().cast::<Align32>().is_aligned());
    Self(_mm256_load_si256(
      data.as_ptr().add(offset) as *const __m256i
    ))
  }

  /// Load at most 32 bytes from the given slice into a vector
  /// by loading it into an intermediate buffer on the stack.
  #[inline(always)]
  unsafe fn load_unaligned_remainder(data: &[u8], offset: usize) -> Self {
    let mut buf = Align32([0; 32]);
    buf.0[..data.len() - offset].copy_from_slice(&data[offset..]);

    Self(unsafe { _mm256_load_si256(buf.0.as_ptr() as *const __m256i) })
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

  #[inline(always)]
  fn cmpeq(self, other: Self) -> Self {
    Self(unsafe { _mm256_cmpeq_epi8(self.0, other.0) })
  }

  #[inline(always)]
  fn movemask(self) -> Mask32 {
    Mask32(unsafe { _mm256_movemask_epi8(self.0) })
  }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct Mask32(i32);

impl Mask32 {
  #[inline(always)]
  fn has_match(self) -> bool {
    self.0 != 0
  }

  #[inline(always)]
  fn first_match_index(self) -> usize {
    self.0.trailing_zeros() as usize
  }
}
