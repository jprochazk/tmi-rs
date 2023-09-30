use beef::lean::Cow;
use std::cell::Cell;
use std::cell::UnsafeCell;

pub struct Unescaped<'a> {
  escaped: Cell<bool>,
  inner: UnsafeCell<Cow<'a, str>>,
}

assert_send!(Unescaped);

impl<'a> Unescaped<'a> {
  pub fn new(s: impl Into<Cow<'a, str>>) -> Self {
    Self {
      escaped: Cell::new(false),
      inner: UnsafeCell::new(s.into()),
    }
  }

  #[inline]
  pub fn get(&self) -> &str {
    if !self.escaped.get() {
      self.unescape();
    }

    unsafe { (*self.inner.get()).as_ref() }
  }

  #[cold]
  fn unescape(&self) {
    unsafe { unescape_in_place(&mut *self.inner.get()) }
    self.escaped.set(true);
  }
}

fn unescape_in_place(value: &mut Cow<'_, str>) {
  #[inline]
  fn actually_unescape(input: &str, start: usize) -> String {
    let mut out = String::with_capacity(input.len());
    out.push_str(&input[..start]);

    let mut escape = false;
    for char in input[start..].chars() {
      match char {
        '\\' if escape => {
          out.push('\\');
          escape = false;
        }
        '\\' => escape = true,
        ':' if escape => {
          out.push(';');
          escape = false;
        }
        's' if escape => {
          out.push(' ');
          escape = false;
        }
        'r' if escape => {
          out.push('\r');
          escape = false;
        }
        'n' if escape => {
          out.push('\n');
          escape = false;
        }
        '⸝' => out.push(','),
        c => out.push(c),
      }
    }

    out
  }

  for i in 0..value.len() {
    if value.as_bytes()[i] == b'\\' {
      *value = Cow::owned(actually_unescape(value, i));
      break;
    }
  }
}

impl<'a> From<&'a str> for Unescaped<'a> {
  fn from(value: &'a str) -> Self {
    Self::new(value)
  }
}

impl<'a> Clone for Unescaped<'a> {
  fn clone(&self) -> Self {
    Self {
      escaped: self.escaped.clone(),
      inner: UnsafeCell::new(unsafe { (*self.inner.get()).clone() }),
    }
  }
}

impl<'a> std::fmt::Debug for Unescaped<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(self.get(), f)
  }
}

impl<'a> std::hash::Hash for Unescaped<'a> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.get().hash(state)
  }
}
impl<'a> std::cmp::PartialEq for Unescaped<'a> {
  fn eq(&self, other: &Self) -> bool {
    self.get() == other.get()
  }
}
impl<'a> std::cmp::Eq for Unescaped<'a> {}
impl<'a> std::cmp::PartialOrd for Unescaped<'a> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}
impl<'a> std::cmp::Ord for Unescaped<'a> {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.get().cmp(other.get())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn unescape_in_place_non_escaped() {
    let mut v = "test".into();
    unescape_in_place(&mut v);
    assert!(v.is_borrowed());
    assert_eq!(v, "test");
  }

  #[test]
  fn unescape_in_place_escaped() {
    let mut v = "\\\\\\n\\s".into();
    unescape_in_place(&mut v);
    assert!(v.is_owned());
    assert_eq!(v, "\\\n ");
  }

  #[test]
  fn unescape_system_msg() {
    let v = Unescaped::new("An\\sanonymous\\suser\\sgifted\\sa\\sTier\\s1\\ssub\\sto\\sDot0422!");
    assert_eq!(v.get(), "An anonymous user gifted a Tier 1 sub to Dot0422!");
  }
}
