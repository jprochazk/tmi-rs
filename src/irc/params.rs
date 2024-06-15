use crate::common::Span;

#[inline(always)]
pub(super) fn parse(src: &str, pos: &usize) -> Option<Span> {
  if !src[*pos..].is_empty() {
    Some(Span::from(*pos..src.len()))
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn params() {
    let data = ":param_a :param_b";
    let params = parse(data, &0).unwrap();
    assert_eq!(params.get(data), data)
  }
}
