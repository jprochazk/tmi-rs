macro_rules! with_scratch {
  ($client:ident, |$scratch:ident| $body:block) => {{
    use ::std::fmt::Write;
    let mut scratch = std::mem::take(&mut $client.scratch);
    let $scratch = &mut scratch;
    let result = { $body };
    scratch.clear();
    $client.scratch = scratch;
    result
  }};
}
