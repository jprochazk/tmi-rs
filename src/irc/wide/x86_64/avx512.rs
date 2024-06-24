use core::arch::x86_64::{
  __m512i, _mm512_cmpeq_epi8_mask, _mm512_load_si512, _mm512_loadu_si512, _mm512_movepi8_mask,
  _mm512_or_si512,
};

#[repr(align(64))]
struct Align64([u8; 64]);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Vector(__m512i);

impl Vector {
  /// Size in bytes.
  pub const SIZE: usize = 64;

  #[inline]
  pub const fn fill(v: u8) -> Self {
    Self(unsafe { core::mem::transmute::<[u8; 64], __m512i>([v; 64]) })
  }

  /// Load 64 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 64 bytes.
  #[inline(always)]
  pub fn load_unaligned(data: &[u8], offset: usize) -> Self {
    unsafe {
      debug_assert!(data[offset..].len() >= 64);
      Self(_mm512_loadu_si512(
        data.as_ptr().add(offset) as *const __m512i
      ))
    }
  }

  /// Load 64 bytes from the given slice into a vector.
  ///
  /// `data[offset..].len()` must be greater than 64 bytes.
  /// The data must be 64-byte aligned.
  #[inline(always)]
  pub fn load_aligned(data: &[u8], offset: usize) -> Self {
    unsafe {
      debug_assert!(data[offset..].len() >= 64);
      debug_assert!(data.as_ptr().add(offset) as usize % 64 == 0);
      Self(_mm512_load_si512(
        data.as_ptr().add(offset) as *const __m512i
      ))
    }
  }

  /// Load at most 64 bytes from the given slice into a vector
  /// by loading it into an intermediate buffer on the stack.
  #[inline(always)]
  pub fn load_unaligned_remainder(data: &[u8], offset: usize) -> Self {
    unsafe {
      let mut buf = Align64([0; 64]);
      buf.0[..data.len() - offset].copy_from_slice(&data[offset..]);

      Self(_mm512_load_si512(buf.0.as_ptr() as *const __m512i))
    }
  }

  #[inline(always)]
  pub fn eq(self, byte: u8) -> Self {
    unsafe { Self(_mm512_cmpeq_epi8_mask(self.0, Self::fill(byte))) }
  }

  #[inline(always)]
  pub fn movemask(self) -> Mask {
    unsafe { Mask(_mm512_movepi8_mask(mask)) }
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
    Self(unsafe { _mm512_or_si512(self.0, rhs.0) })
  }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Mask(u64);

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
    self.0 &= !(0xffff_ffff_ffff_ffff >> (63 - m.0));
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
