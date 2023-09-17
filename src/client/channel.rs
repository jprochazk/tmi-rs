use super::write::SameMessageBypass;

pub struct Channel {
  pub name: String,
  pub smb: SameMessageBypass,
}
