use core::arch::x86_64::{
  __m128i, _mm_cmpeq_epi8, _mm_load_si128, _mm_loadu_si128, _mm_movemask_epi8, _mm_or_si128,
};

#[repr(align(16))]
struct Align16([u8; 16]);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vector(__m128i);

impl Vector {
  /// Size in bytes.
  pub const SIZE: usize = 16;

  #[inline]
  pub const fn fill(v: u8) -> Self {
    Self(unsafe { core::mem::transmute::<[u8; 16], __m128i>([v; 16]) })
  }

  /// Load 16 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 16 bytes.
  #[inline(always)]
  pub fn load_unaligned(data: &[u8], offset: usize) -> Self {
    unsafe {
      debug_assert!(data[offset..].len() >= 16);
      Self(_mm_loadu_si128(data.as_ptr().add(offset) as *const __m128i))
    }
  }

  /// Load 16 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 16 bytes.
  /// The data must be 16-byte aligned.
  #[inline(always)]
  pub fn load_aligned(data: &[u8], offset: usize) -> Self {
    unsafe {
      debug_assert!(data[offset..].len() >= 16);
      debug_assert!(data.as_ptr().add(offset) as usize % 16 == 0);
      Self(_mm_load_si128(data.as_ptr().add(offset) as *const __m128i))
    }
  }

  /// Load at most 16 bytes from the given slice into a vector
  /// by loading it into an intermediate buffer on the stack.
  #[inline(always)]
  pub fn load_unaligned_remainder(data: &[u8], offset: usize) -> Self {
    unsafe {
      let mut buf = Align16([0; 16]);
      buf.0[..data.len() - offset].copy_from_slice(&data[offset..]);

      Self(_mm_load_si128(buf.0.as_ptr() as *const __m128i))
    }
  }

  /// Compare 16 8-bit elements in `self` against `other`, leaving a `1` in each
  #[inline(always)]
  pub fn eq(self, byte: u8) -> Self {
    unsafe { Self(_mm_cmpeq_epi8(self.0, Self::fill(byte).0)) }
  }

  #[inline(always)]
  pub fn movemask(self) -> Mask {
    unsafe {
      let value = std::mem::transmute::<i32, u32>(_mm_movemask_epi8(self.0));
      Mask(value)
    }
  }

  pub const SUPPORTS_MOVEMASK_WILL_HAVE_NON_ZERO: bool = false;

  #[inline(always)]
  pub fn movemask_will_have_non_zero(self) -> bool {
    unreachable!("unsupported")
  }
}

impl std::ops::BitOr for Vector {
  type Output = Self;

  #[inline(always)]
  fn bitor(self, rhs: Self) -> Self {
    Self(unsafe { _mm_or_si128(self.0, rhs.0) })
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
  pub fn first_match(&self) -> Match {
    Match(self.0.trailing_zeros() as usize)
  }

  /// Clear all bits up to and including `m`.
  #[inline(always)]
  pub fn clear_to(&mut self, m: Match) {
    self.0 &= !(0xffff_ffff >> (31 - m.0));
  }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Match(usize);

impl Match {
  #[inline(always)]
  pub fn as_index(&self) -> usize {
    self.0
  }
}
