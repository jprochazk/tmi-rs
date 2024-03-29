use std::borrow::Borrow;
use std::ops::Deref;

/// Channel name known to be prefixed by `#`.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ChannelRef(str);

impl ChannelRef {
  /// Get the string value of the channel name.
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Parse a string into a channel name.
  ///
  /// The channel name must begin with a `#` character.
  pub fn parse(s: &str) -> Result<&Self, InvalidChannelName> {
    match s.starts_with('#') {
      true => Ok(Self::from_unchecked(s)),
      false => Err(InvalidChannelName),
    }
  }

  pub(crate) fn from_unchecked(s: &str) -> &Self {
    // # Safety:
    // - `Self` is `repr(transparent)` and only holds a single `str` field,
    //   therefore the layout of `Self` is the same as `str`, and it's
    //   safe to transmute between the two
    unsafe { std::mem::transmute(s) }
  }
}

impl Deref for ChannelRef {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl AsRef<str> for ChannelRef {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

impl AsRef<ChannelRef> for ChannelRef {
  fn as_ref(&self) -> &ChannelRef {
    self
  }
}

impl Borrow<str> for ChannelRef {
  fn borrow(&self) -> &str {
    &self.0
  }
}

impl std::fmt::Debug for ChannelRef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Channel").field(&self.as_str()).finish()
  }
}

impl std::fmt::Display for ChannelRef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0)
  }
}

impl ToOwned for ChannelRef {
  type Owned = Channel;

  fn to_owned(&self) -> Self::Owned {
    Channel::from_unchecked(self.as_str().to_owned())
  }
}

/// Channel name known to be prefixed by `#`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Channel(String);

impl Channel {
  /// Get the string value of the channel name.
  pub fn as_str(&self) -> &str {
    self.0.as_str()
  }

  /// Parse a string into a channel name.
  ///
  /// The channel name must begin with a `#` character.
  pub fn parse(s: String) -> Result<Self, InvalidChannelName> {
    match s.starts_with('#') {
      true => Ok(Self(s)),
      false => Err(InvalidChannelName),
    }
  }

  pub(crate) fn from_unchecked(s: String) -> Self {
    Self(s)
  }
}

impl Deref for Channel {
  type Target = String;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl AsRef<str> for Channel {
  fn as_ref(&self) -> &str {
    self.0.as_ref()
  }
}

impl AsRef<ChannelRef> for Channel {
  fn as_ref(&self) -> &ChannelRef {
    ChannelRef::from_unchecked(self.0.as_str())
  }
}

impl Borrow<str> for Channel {
  fn borrow(&self) -> &str {
    self.0.borrow()
  }
}

impl Borrow<ChannelRef> for Channel {
  fn borrow(&self) -> &ChannelRef {
    ChannelRef::from_unchecked(self.0.borrow())
  }
}

impl std::fmt::Display for Channel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.0)
  }
}

/// Failed to parse a channel name.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvalidChannelName;
impl std::fmt::Display for InvalidChannelName {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("channel name is missing \"#\" prefix")
  }
}
impl std::error::Error for InvalidChannelName {}

static_assert_send!(ChannelRef);
static_assert_sync!(ChannelRef);

static_assert_send!(Channel);
static_assert_sync!(Channel);

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_channel() {
    assert_eq!(
      ChannelRef::parse("#test"),
      Ok(ChannelRef::from_unchecked("#test"))
    );
    assert_eq!(ChannelRef::parse("test"), Err(InvalidChannelName));
    assert_eq!(
      Channel::parse("#test".into()),
      Ok(Channel::from_unchecked("#test".into()))
    );
    assert_eq!(Channel::parse("test".into()), Err(InvalidChannelName));
  }
}

#[cfg(feature = "serde")]
mod _serde {
  use super::*;
  use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

  impl<'de: 'src, 'src> Deserialize<'de> for &'src ChannelRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: Deserializer<'de>,
    {
      ChannelRef::parse(<&str as Deserialize<'de>>::deserialize(deserializer)?)
        .map_err(de::Error::custom)
    }
  }

  impl<'ser> Serialize for &'ser ChannelRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: Serializer,
    {
      <&str as Serialize>::serialize(&self.as_str(), serializer)
    }
  }

  impl<'de> Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: Deserializer<'de>,
    {
      Channel::parse(<String as Deserialize<'de>>::deserialize(deserializer)?)
        .map_err(de::Error::custom)
    }
  }

  impl Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: Serializer,
    {
      <String as Serialize>::serialize(&self.0, serializer)
    }
  }
}
