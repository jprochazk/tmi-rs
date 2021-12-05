pub type Message = tokio_tungstenite::tungstenite::Message;

/* use serde::{de::IntoDeserializer, Deserialize, Deserializer};

#[derive(Debug, Clone, Copy)]
pub enum Topic {
  Bits(usize),
  ChannelPoints(usize),
}

impl<'de> Deserialize<'de> for Topic {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let s = String::deserialize(deserializer)?;
    if s.starts_with("channel-bits-events-v2") {

    }
  }
}

#[derive(Clone, Debug, Copy, Deserialize)]
pub enum ResponseError {
  #[serde(rename = "ERR_BADMESSAGE")]
  BadMessage,
  #[serde(rename = "ERR_BADAUTH")]
  BadAuth,
  #[serde(rename = "ERR_BADTOPIC")]
  BadTopic,
  #[serde(rename = "ERR_SERVER")]
  Server,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
  #[serde(rename = "PONG")]
  Pong,
  #[serde(rename = "RECONNECT")]
  Reconnect,
  #[serde(rename = "RESPONSE")]
  Response {
    nonce: String,
    #[serde(deserialize_with = "empty_string_as_none")]
    error: Option<ResponseError>,
  },
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
  D: Deserializer<'de>,
  T: Deserialize<'de>,
{
  let opt = Option::<String>::deserialize(de)?;
  let opt = opt.as_deref();
  match opt {
    None | Some("") => Ok(None),
    Some(s) => T::deserialize(s.into_deserializer()).map(Some),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn deserialize_pong() {
    const msg: &str = r#"{
      "type": "PING"
    }"#;
  }

  #[test]
  fn deserialize_reconnect() {
    const msg: &str = r#"{
      "type": "RECONNECT"
    }"#;
  }

  #[test]
  fn deserialize_response() {
    const msg: &str = r#"{
      "type": "RESPONSE",
      "nonce": "44h1k13746815ab1r2",
      "data": {
        "topics": ["channel-bits-events-v1.44322889"],
        "auth_token": "cfabdegwdoklmawdzdo98xt2fo512y"
      }
    }"#;
  }
} */
