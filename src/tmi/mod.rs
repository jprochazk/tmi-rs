//! * [`conn`](./conn) - TMI connection utility
//! * [`irc`](./irc) - parsing raw IRC messages, with Twitch-specific extensions (not RFC2812 compliant)
//! * [`parse`](./parse) - parsing Twitch messages from raw IRC messages
//! * [`write`](./write) - writing Twitch IRC messages

pub mod conn;
pub mod irc;
pub mod parse;
pub(crate) mod util;
pub mod write;

pub use conn::connect;
pub use conn::Config;
pub use conn::Connection;
pub use conn::Login;
pub use parse::Message;
