use std::sync::Arc;

use super::write::SameMessageBypass;

#[derive(Clone)]
pub struct ChannelSender(Arc<ChannelSenderInner>);

struct ChannelSenderInner {
  name: String,
  sender: tokio::sync::mpsc::Sender<String>,
}

pub struct Channel {
  pub name: String,
  pub smb: SameMessageBypass,
}
