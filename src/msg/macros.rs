macro_rules! generate_getters {
  {
    $(<$($L:lifetime)*>)? for $T:ty as $self:ident {
      $(
        $(#[$meta:meta])*
        $field:ident -> $R:ty $(= $e:expr)?
      ),* $(,)?
    }
  } => {
    impl$(<$($L)*>)? $T {
      $(
        #[inline]
        $(#[$meta])*
        pub fn $field(&$self) -> $R {
          generate_getters!(@getter $self $field $($e)?)
        }
      )*
    }
  };

  (@getter $self:ident $field:ident $e:expr) => ($e);
  (@getter $self:ident $field:ident) => ($self.$field.clone());
}

#[cfg(test)]
pub(crate) fn _parse_irc<'src, T: crate::msg::FromIrc<'src>>(input: &'src str) -> T {
  let raw = crate::irc::IrcMessageRef::parse(input).unwrap();
  <T as crate::msg::FromIrc>::from_irc(raw).unwrap()
}

#[cfg(test)]
macro_rules! assert_irc_snapshot {
  ($T:ty, $input:literal,) => {
    assert_irc_snapshot!($T, $input)
  };
  ($T:ty, $input:literal) => {{
    let f = $crate::msg::macros::_parse_irc::<$T>;
    ::insta::assert_debug_snapshot!(f($input))
  }};
}

#[cfg(all(test, feature = "serde"))]
macro_rules! assert_irc_roundtrip {
  ($T:ty, $input:literal,) => {
    assert_serde_roundtrip!($T, $input)
  };
  ($T:ty, $input:literal) => {{
    let original = $crate::msg::macros::_parse_irc::<$T>($input);
    let serialized = ::serde_json::to_string(&original).expect("failed to serialize");
    let deserialized = ::serde_json::from_str(&serialized).expect("failed to deserialize");
    assert_eq!(original, deserialized, "roundtrip failed");
  }};
}
