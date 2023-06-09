#[macro_export]
macro_rules! whitelist {
  [$($tag:ident),*] => (
    unsafe {
      $crate::Whitelist::new({
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
