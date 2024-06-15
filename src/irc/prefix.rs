use crate::common::Span;

#[derive(Debug, Clone, Copy)]
pub(super) struct RawPrefix {
  nick: Option<Span>,
  user: Option<Span>,
  host: Span,
}

impl RawPrefix {
  pub(super) fn get<'src>(&self, src: &'src str) -> Prefix<'src> {
    Prefix {
      nick: self.nick.map(|span| &src[span]),
      user: self.user.map(|span| &src[span]),
      host: &src[self.host],
    }
  }
}

// TODO: have prefix only be two variants: `User` and `Host`
/// A message prefix.
///
/// ```text,ignore
/// :nick!user@host
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prefix<'src> {
  /// The `nick` part of the prefix.
  pub nick: Option<&'src str>,
  /// The `user` part of the prefix.
  pub user: Option<&'src str>,
  /// The `host` part of the prefix.
  pub host: &'src str,
}

impl<'src> std::fmt::Display for Prefix<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match (self.nick, self.user, self.host) {
      (Some(nick), Some(user), host) => write!(f, "{nick}!{user}@{host}"),
      (Some(nick), None, host) => write!(f, "{nick}@{host}"),
      (None, None, host) => write!(f, "{host}"),
      _ => Ok(()),
    }
  }
}

/// `:nick!user@host <rest>`
#[inline(always)]
pub(super) fn parse(src: &str, pos: &mut usize) -> Option<RawPrefix> {
  if !src[*pos..].starts_with(':') {
    return None;
  }

  // :host <rest>
  // :nick@host <rest>
  // :nick!user@host <rest>
  let bytes = src.as_bytes();

  let start = *pos + 1;
  let mut host_start = start;
  let mut nick = None;
  let mut nick_end = None;
  let mut user = None;
  for i in start..bytes.len() {
    match unsafe { *bytes.get_unchecked(i) } {
      b' ' => {
        let host = Span::from(host_start..i);
        *pos = i + 1;
        return Some(RawPrefix { nick, user, host });
      }
      b'@' => {
        host_start = i + 1;
        if let Some(nick_end) = nick_end {
          user = Some(Span::from(nick_end + 1..i));
        } else {
          nick = Some(Span::from(start..i));
        }
      }
      b'!' => {
        nick = Some(Span::from(start..i));
        nick_end = Some(i);
      }
      _ => {}
    }
  }

  None
}
