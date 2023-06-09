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
      $crate::Whitelist::<{$crate::__count!($($tag)*)}, _>::new({
        #[allow(unused_variables)]
        #[inline]
        |map: &mut $crate::Tags, tag: $crate::Tag, value: &str| {
          match tag {
            $($crate::Tag::$tag => {map.insert(tag, value);})*
            _ => {}
          };
        }
      })
    }
  )
}
