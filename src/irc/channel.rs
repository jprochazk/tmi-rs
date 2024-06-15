use crate::common::Span;

/// #channel <rest>
#[inline(always)]
pub(super) fn parse(src: &str, pos: &mut usize) -> Option<Span> {
  match src[*pos..].starts_with('#') {
    true => {
      let start = *pos;
      match src[start..].find(' ') {
        Some(end) => {
          let end = start + end;
          *pos = end + 1;
          Some(Span::from(start..end))
        }
        None => {
          let end = src.len();
          *pos = end;
          Some(Span::from(start..end))
        }
      }
    }
    false => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn channel() {
    let data = "#channel <rest>";
    let mut pos = 0;

    let channel = parse(data, &mut pos).unwrap();
    assert_eq!(channel.get(data), "#channel");
    assert_eq!(&data[pos..], "<rest>");
  }
}
