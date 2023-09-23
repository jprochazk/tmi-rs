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
      $crate::msg::Whitelist::<{$crate::__count!($($tag)*)}, _>::new({
        #[allow(unused_variables)]
        #[inline]
        |src: &str, map: &mut $crate::msg::RawTags, tag: $crate::msg::Span, value: $crate::msg::Span| {
          match src[tag].as_bytes() {
            $($crate::msg::tags::$tag => {map.push($crate::msg::RawTagPair($crate::msg::RawTag::$tag, value));})*
            _ => {}
          };
        }
      })
    }
  )
}
