//! Message parsing/writing module
//!
//! * [`irc`](./irc) - parsing raw IRC messages, with Twitch-specific extensions
//!   (not RFC2812 compliant)
//! * [`tmi`](./twitch) - parsing Twitch-specific commands (PRIVMSG, ROOMSTATE,
//!   USERNOTICE, etc.)
//! * [`conn`](./conn) - TMI connection utility
#![feature(str_split_once)]

pub mod conn;
pub mod irc;
pub mod tmi;
pub(crate) mod util;

pub use conn::connect;
pub use conn::Config;
pub use conn::Connection;
pub use tmi::parse::Capability;
pub use tmi::parse::Clearchat;
pub use tmi::parse::Clearmsg;
pub use tmi::parse::GlobalUserState;
pub use tmi::parse::HostTarget;
pub use tmi::parse::Join;
pub use tmi::parse::Notice;
pub use tmi::parse::Part;
pub use tmi::parse::Ping;
pub use tmi::parse::Pong;
pub use tmi::parse::Privmsg;
pub use tmi::parse::Reconnect;
pub use tmi::parse::RoomState;
pub use tmi::parse::UserNotice;
pub use tmi::parse::UserState;
pub use tmi::parse::Whisper;
pub use tmi::Message;
