use std::{
  convert::{AsRef, From},
  fmt,
  fmt::{Debug, Display, Formatter},
  hash::{Hash, Hasher},
};
use std::{slice, str};

/// This type is *deeply* unsafe. It exists to bypass limitations of Rust related to self-referential structs.
/// The alternative is allocating everywhere, which leads to poor performance.
///
/// SAFETY: Must not outlive the `String` it points to. The `String`'s memory must also not be moved.
#[derive(Clone, Copy)]
pub(crate) struct UnsafeSlice {
  ptr: *const u8,
  len: usize,
}

impl UnsafeSlice {
  pub fn as_str<'a>(&self) -> &'a str {
    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.ptr, self.len)) }
  }
}

impl From<&str> for UnsafeSlice {
  fn from(value: &str) -> UnsafeSlice {
    UnsafeSlice {
      ptr: value.as_ptr(),
      len: value.len(),
    }
  }
}

impl AsRef<str> for UnsafeSlice {
  fn as_ref(&self) -> &str {
    str::from_utf8(unsafe { slice::from_raw_parts(self.ptr, self.len) }).unwrap()
  }
}
impl Debug for UnsafeSlice {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    Debug::fmt(AsRef::<str>::as_ref(self), f)
  }
}
impl Display for UnsafeSlice {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    Display::fmt(AsRef::<str>::as_ref(self), f)
  }
}
impl Eq for UnsafeSlice {}
impl PartialEq<UnsafeSlice> for UnsafeSlice {
  fn eq(&self, other: &UnsafeSlice) -> bool {
    (AsRef::<str>::as_ref(self)).eq(AsRef::<str>::as_ref(other))
  }
}
impl Hash for UnsafeSlice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    (AsRef::<str>::as_ref(self)).hash(state)
  }
}
impl Default for UnsafeSlice {
  fn default() -> Self {
    "".into()
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  #[test]
  fn unsafeslice_usable_in_hash_map() {
    let data = "Hello".to_string();
    let slice: UnsafeSlice = (&data[..]).into();

    let mut map = HashMap::<UnsafeSlice, UnsafeSlice>::new();
    map.insert(slice, slice);
    assert_eq!(map.get(&slice).unwrap(), &slice);
  }
}
