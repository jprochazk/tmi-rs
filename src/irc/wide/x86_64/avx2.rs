use core::arch::x86_64::{
  __m256i, _mm256_cmpeq_epi8, _mm256_load_si256, _mm256_loadu_si256, _mm256_movemask_epi8,
};

#[repr(align(32))]
struct Align32([u8; 32]);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vector(__m256i);

impl Vector {
  /// Size in bytes.
  pub const SIZE: usize = 32;

  #[inline]
  pub const fn fill(v: u8) -> Self {
    Self(unsafe { core::mem::transmute::<[u8; 32], __m256i>([v; 32]) })
  }

  /// Load 32 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 32 bytes.
  #[inline(always)]
  pub fn load_unaligned(data: &[u8], offset: usize) -> Self {
    unsafe {
      debug_assert!(data[offset..].len() >= 32);
      Self(_mm256_loadu_si256(
        data.as_ptr().add(offset) as *const __m256i
      ))
    }
  }

  /// Load 32 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 32 bytes.
  /// The data must be 32-byte aligned.
  #[inline(always)]
  pub fn load_aligned(data: &[u8], offset: usize) -> Self {
    unsafe {
      debug_assert!(data[offset..].len() >= 32);
      debug_assert!(data.as_ptr().add(offset) as usize % 32 == 0);
      Self(_mm256_load_si256(
        data.as_ptr().add(offset) as *const __m256i
      ))
    }
  }

  /// Load at most 32 bytes from the given slice into a vector
  /// by loading it into an intermediate buffer on the stack.
  #[inline(always)]
  pub fn load_unaligned_remainder(data: &[u8], offset: usize) -> Self {
    unsafe {
      let mut buf = Align32([0; 32]);
      buf.0[..data.len() - offset].copy_from_slice(&data[offset..]);

      Self(_mm256_load_si256(buf.0.as_ptr() as *const __m256i))
    }
  }

  #[inline(always)]
  pub fn eq(self, byte: u8) -> Self {
    unsafe { Self(_mm256_cmpeq_epi8(self.0, Self::fill(byte).0)) }
  }

  #[inline(always)]
  pub fn movemask(self) -> Mask {
    unsafe {
      let value = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(self.0));
      Mask(value)
    }
  }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Mask(u32);

impl Mask {
  #[inline(always)]
  pub fn has_match(&self) -> bool {
    self.0 != 0
  }

  #[inline(always)]
  pub fn first_match(&self) -> usize {
    self.0.trailing_zeros() as usize
  }

  #[inline(always)]
  pub fn clear_to(&mut self, bit: usize) {
    // clear all bits up to and including `bit`
    self.0 &= !(0xffff_ffff >> (31 - bit));
  }
}
