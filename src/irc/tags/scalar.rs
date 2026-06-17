use super::*;

pub(crate) fn parse(src: &str, pos: &mut usize) -> Option<RawTags> {
  let src = src[*pos..].strip_prefix('@')?.as_bytes();

  let mut tags = Array::<128, TagPair>::new();

  let mut state = State::Key { key_start: 0 };
  let mut offset = 0;
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
          tags.push(TagPair {
            key_start: key_start as u32 + 1,
            key_len: (key_end - key_start) as u16,
            val_len: (offset - (key_end + 1)) as u16,
          });
          state = State::Key {
            key_start: offset + 1,
          };
        }
        State::Key { key_start } => {
          tags.push(TagPair {
            key_start: key_start as u32 + 1,
            key_len: (offset - key_start) as u16,
            val_len: 0,
          });
          state = State::Key {
            key_start: offset + 1,
          }
        }
      },
      b' ' => {
        match state {
          State::Value { key_start, key_end } => {
            tags.push(TagPair {
              key_start: key_start as u32 + 1,
              key_len: (key_end - key_start) as u16,
              val_len: (offset - (key_end + 1)) as u16,
            });
          }
          State::Key { key_start } => {
            tags.push(TagPair {
              key_start: key_start as u32 + 1,
              key_len: (offset - key_start) as u16,
              val_len: 0,
            });
          }
        }
        break;
      }
      _ => {}
    }

    offset += 1;
  }

  *pos += offset + 2; // skip '@' + space

  Some(RawTags(tags.to_vec()))
}

#[derive(Clone, Copy)]
enum State {
  Key { key_start: usize },
  Value { key_start: usize, key_end: usize },
}
