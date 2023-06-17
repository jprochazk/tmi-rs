use crate::{leak, ParsedTags, Prefix, Tags, Whitelist};

/// `@a=a;b=b;c= :<rest>`
pub fn parse_tags<'src, const IC: usize, F>(
  remainder: &'src str,
  whitelist: &Whitelist<IC, F>,
) -> (Option<ParsedTags<'static>>, &'src str)
where
  F: for<'a> Fn(&'a mut Tags<'static>, &'static str, &'static str),
{
  if let Some(remainder) = remainder.strip_prefix('@') {
    let mut tags = Tags::with_capacity(IC);
    let mut key = (0, 0);
    let mut value = (0, 0);
    let mut end = 0;

    let bytes = remainder.as_bytes();
    for i in 0..bytes.len() {
      match unsafe { *bytes.get_unchecked(i) } {
        b' ' if unsafe { *bytes.get_unchecked(i + 1) } == b':' => {
          value.1 = i;
          if key.1 - key.0 > 0 {
            let tag = unsafe { leak(&remainder[key.0..key.1]) };
            let value = unsafe { leak(&remainder[value.0..value.1]) };
            whitelist.maybe_insert(&mut tags, tag, value);
          }
          end = i;
          break;
        }
        b'=' if value.1 <= key.1 => {
          key.1 = i;
          value.0 = i + 1;
          value.1 = i + 1;
        }
        b';' => {
          value.1 = i;

          let tag = unsafe { leak(&remainder[key.0..key.1]) };
          let value = unsafe { leak(&remainder[value.0..value.1]) };
          whitelist.maybe_insert(&mut tags, tag, value);

          key.0 = i + 1;
          key.1 = i + 1;
        }
        _ => {}
      }
    }

    (Some(tags.into_boxed_slice()), &remainder[end + 1..])
  } else {
    (None, remainder)
  }
}

/// `:nick!user@host <rest>`
pub fn parse_prefix(remainder: &str) -> (Option<Prefix<'static>>, &str) {
  if let Some(remainder) = remainder.strip_prefix(':') {
    // :host <rest>
    // :nick@host <rest>
    // :nick!user@host <rest>
    let bytes = remainder.as_bytes();

    let mut host_start = None;
    let mut nick = None;
    let mut nick_end = None;
    let mut user = None;
    for i in 0..bytes.len() {
      match unsafe { *bytes.get_unchecked(i) } {
        b' ' => {
          let host_range = match host_start {
            Some(host_start) => host_start..i,
            None => 0..i,
          };
          let host = unsafe { &*(&remainder[host_range] as *const _) };

          return (Some(Prefix { nick, user, host }), &remainder[i + 1..]);
        }
        b'@' => {
          host_start = Some(i + 1);
          if let Some(nick_end) = nick_end {
            user = Some(unsafe { &*(&remainder[nick_end + 1..i] as *const _) });
          } else {
            nick = Some(unsafe { &*(&remainder[..i] as *const _) });
          }
        }
        b'!' => {
          nick = Some(unsafe { &*(&remainder[..i] as *const _) });
          nick_end = Some(i);
        }
        _ => {}
      }
    }

    (None, remainder)
  } else {
    (None, remainder)
  }
}
