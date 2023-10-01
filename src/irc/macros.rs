#[doc(hidden)]
#[macro_export]
macro_rules! __count {
  () => (0usize);
  ($x:tt $($xs:tt)*) => (1usize + $crate::__count!($($xs)*));
}

/// Constructs a whitelist from a list of tags names.
///
/// The tag names are the PascalCase variants of the [`Tag`][Tag] enum.
///
/// [Tag]: crate::irc::Tag
#[macro_export]
macro_rules! whitelist {
  [$($tag:ident),*] => (
    $crate::irc::Whitelist::<{$crate::__count!($($tag)*)}, _>::new({
      #[allow(unused_variables)]
      #[inline]
      |src: &str, map: &mut $crate::irc::RawTags, tag: $crate::common::Span, value: $crate::common::Span| {
        match src[tag].as_bytes() {
          $($crate::irc::tags::$tag => {map.push($crate::irc::RawTagPair($crate::irc::RawTag::$tag, value));})*
          _ => {}
        };
      }
    })
  )
}
