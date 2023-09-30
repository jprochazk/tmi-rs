use std::cell::RefCell;

/// Channel name known to be prefixed by `#`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Channel<'src>(&'src str);

impl<'src> Channel<'src> {
  pub fn as_str(&self) -> &'src str {
    self.0
  }

  pub fn parse(s: &'src str) -> Result<Self, InvalidChannelName> {
    match s.starts_with('#') {
      true => Ok(Self(s)),
      false => Err(InvalidChannelName),
    }
  }

  pub(crate) fn from_unchecked(s: &'src str) -> Self {
    Self(s)
  }
}

impl<'src> std::fmt::Display for Channel<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.0)
  }
}

impl<'src> TryFrom<&'src str> for Channel<'src> {
  type Error = InvalidChannelName;

  fn try_from(value: &'src str) -> Result<Self, Self::Error> {
    Self::parse(value)
  }
}

#[derive(Debug)]
pub struct InvalidChannelName;
impl std::fmt::Display for InvalidChannelName {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("channel name is missing \"#\" prefix")
  }
}
impl std::error::Error for InvalidChannelName {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
  pub start: u32,
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

#[doc(hidden)]
pub mod unescaped;
