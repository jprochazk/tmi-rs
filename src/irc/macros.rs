#[doc(hidden)]
#[macro_export]
macro_rules! __count {
  () => (0usize);
  ($x:tt $($xs:tt)*) => (1usize + $crate::__count!($($xs)*));
}

#[macro_export]
macro_rules! whitelist {
  [$($tag:ident),*] => (
    unsafe {
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
    }
  )
}
