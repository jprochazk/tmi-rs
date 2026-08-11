use super::*;

pub(crate) fn parse(src: &str, pos: &mut usize) -> Option<RawTags> {
  let src = src[*pos..].strip_prefix('@')?.as_bytes();
  if src.first() == Some(&b' ') {
    return None;
  }

  let mut tags = Array::<128, TagPair>::new();

  let mut state = State::Key { key_start: 0 };
  let mut offset = 0;
  let mut complete = false;
  while offset < src.len() {
    let c = src[offset];
    match c {
      b'=' => {
        if let State::Key { key_start } = state {
          state = State::Value {
            key_start,
            key_end: offset,
          };
        }
      }
      b';' => match state {
        State::Value { key_start, key_end } => {
          tags.push(TagPair::valued::<true>(key_start, key_end, offset)?)?;
          state = State::Key {
            key_start: offset + 1,
          };
        }
        State::Key { key_start } => {
          tags.push(TagPair::valueless::<true>(key_start, offset)?)?;
          state = State::Key {
            key_start: offset + 1,
          };
        }
      },
      b' ' => {
        match state {
          State::Value { key_start, key_end } => {
            tags.push(TagPair::valued::<true>(key_start, key_end, offset)?)?;
          }
          State::Key { key_start } => {
            tags.push(TagPair::valueless::<true>(key_start, offset)?)?;
          }
        }
        complete = true;
        break;
      }
      _ => {}
    }

    offset += 1;
  }

  if !complete {
    return None;
  }

  *pos += offset + 2; // skip '@' + space

  Some(RawTags(tags.to_vec()))
}

#[derive(Clone, Copy)]
enum State {
  Key { key_start: usize },
  Value { key_start: usize, key_end: usize },
}
