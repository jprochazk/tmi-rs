//! Random types and utilties used by the library.

pub mod channel;

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;

pub use channel::{Channel, ChannelRef, InvalidChannelName};

/// This type is like a [`Range`][std::ops::Range],
/// only smaller, and also implements `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
  /// The start index, inclusive.
  pub start: u32,

  /// The end index, exclusive.
  pub end: u32,
}

impl Span {
  #[allow(dead_code)]
  #[doc(hidden)]
  #[inline]
  pub(crate) fn get<'src>(&self, src: &'src str) -> &'src str {
    &src[*self]
  }
}

impl From<std::ops::Range<usize>> for Span {
  #[inline]
  fn from(value: std::ops::Range<usize>) -> Self {
    Span {
      start: value.start as u32,
      end: value.end as u32,
    }
  }
}

impl From<Span> for std::ops::Range<usize> {
  #[inline]
  fn from(value: Span) -> Self {
    value.start as usize..value.end as usize
  }
}

impl std::ops::Index<Span> for str {
  type Output = <str as std::ops::Index<std::ops::Range<usize>>>::Output;

  #[inline]
  fn index(&self, index: Span) -> &Self::Output {
    self.index(std::ops::Range::from(index))
  }
}

#[doc(hidden)]
pub struct Join<I, S>(RefCell<Option<I>>, S);

impl<I, S> std::fmt::Display for Join<I, S>
where
  // TODO: get rid of this `Clone` bound by doing `peek`
  // manually
  I: Iterator,
  <I as Iterator>::Item: std::fmt::Display,
  S: std::fmt::Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Some(iter) = self.0.borrow_mut().take() else {
      return Err(std::fmt::Error);
    };

    let sep = &self.1;
    let mut peekable = iter.peekable();
    while let Some(item) = peekable.next() {
      write!(f, "{item}")?;
      if peekable.peek().is_some() {
        write!(f, "{sep}")?;
      }
    }
    Ok(())
  }
}

#[doc(hidden)]
pub trait JoinIter: Sized {
  fn join<Sep>(self, sep: Sep) -> Join<Self, Sep>;
}

impl<Iter> JoinIter for Iter
where
  Iter: Sized + Iterator,
{
  fn join<Sep>(self, sep: Sep) -> Join<Self, Sep> {
    Join(RefCell::new(Some(self)), sep)
  }
}

pub(crate) fn maybe_unescape<'a>(value: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
  let mut value: Cow<'_, str> = value.into();
  for i in 0..value.len() {
    if value.as_bytes()[i] == b'\\' {
      value = Cow::Owned(actually_unescape(&value, i));
      break;
    }
  }
  value
}

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

/// Cow-equivalent type which is used to bypass the deserialize
/// restrictions for `Cow<'a, T>` where `T` is not `str`...
pub(crate) enum MaybeOwned<'a, T: ?Sized + ToOwned> {
  Ref(&'a T),
  Own(T::Owned),
}

impl<'a, T: PartialEq> PartialEq for MaybeOwned<'a, T>
where
  T: ?Sized,
  T: ToOwned,
  T::Owned: AsRef<T>,
{
  fn eq(&self, other: &Self) -> bool {
    self.as_ref() == other.as_ref()
  }
}

impl<'a, T: Eq> Eq for MaybeOwned<'a, T>
where
  T: ?Sized,
  T: ToOwned,
  T::Owned: AsRef<T>,
{
}

impl<'a, T> Deref for MaybeOwned<'a, T>
where
  T: ?Sized,
  T: ToOwned,
  T::Owned: AsRef<T>,
{
  type Target = T;

  fn deref(&self) -> &Self::Target {
    match self {
      MaybeOwned::Ref(v) => v,
      MaybeOwned::Own(v) => v.as_ref(),
    }
  }
}

impl<'a, T> AsRef<T> for MaybeOwned<'a, T>
where
  T: ?Sized,
  T: ToOwned,
  T::Owned: AsRef<T>,
{
  fn as_ref(&self) -> &T {
    match self {
      MaybeOwned::Ref(v) => v,
      MaybeOwned::Own(v) => v.as_ref(),
    }
  }
}

impl<T> Debug for MaybeOwned<'_, T>
where
  T: ?Sized,
  T: ToOwned,
  T: Debug,
  T::Owned: Debug,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Ref(arg0) => Debug::fmt(arg0, f),
      Self::Own(arg0) => Debug::fmt(arg0, f),
    }
  }
}

impl<T> Clone for MaybeOwned<'_, T>
where
  T: ?Sized,
  T: ToOwned,
  T::Owned: Clone,
{
  fn clone(&self) -> Self {
    match self {
      Self::Ref(arg0) => Self::Ref(*arg0),
      Self::Own(arg0) => Self::Own(<T::Owned>::clone(arg0)),
    }
  }
}

#[cfg(feature = "serde")]
mod _serde {
  use super::*;
  use serde::{de, Deserialize, Serialize};

  impl<'a> Serialize for MaybeOwned<'a, ChannelRef> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::Serializer,
    {
      self.as_ref().serialize(serializer)
    }
  }

  impl<'de: 'a, 'a> Deserialize<'de> for MaybeOwned<'a, ChannelRef> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: serde::Deserializer<'de>,
    {
      match <Cow<'a, str>>::deserialize(deserializer)? {
        Cow::Borrowed(v) => Ok(MaybeOwned::Ref(
          ChannelRef::parse(v).map_err(de::Error::custom)?,
        )),
        Cow::Owned(v) => Ok(MaybeOwned::Own(
          Channel::parse(v).map_err(de::Error::custom)?,
        )),
      }
    }
  }
}
