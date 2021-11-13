use std::fmt::{self, Write};

use chrono::Duration;

/*
TODO: write tests
✅ /w {USERNAME} {MESSAGE}
✅ /me
✅ /clear
✅ /timeout <user> [duration {SECONDS} = 600]
✅ /untimeout <user>
✅ /ban <user>
✅ /unban <user>
✅ /uniquechat
✅ /uniquechatoff
✅ /subscribers
✅ /subscribersoff
✅ /emoteonly
✅ /emoteonlyoff
✅ /slow <{SECONDS}>
✅ /slowoff
✅ /followers {TIME}
✅ /followersoff
*/

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SameMessageBypass {
    flag: u8,
}
impl SameMessageBypass {
    pub fn get(&mut self) -> &'static str {
        match self.flag {
            0 => {
                self.flag = 1;
                ""
            }
            1 => {
                self.flag = 0;
                "⠀"
            }
            _ => unreachable!(),
        }
    }
}
impl Default for SameMessageBypass {
    fn default() -> Self {
        SameMessageBypass { flag: 0 }
    }
}

struct NoAllocWrite<'a>(&'a mut String);
impl<'a> fmt::Write for NoAllocWrite<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.len() > self.0.capacity() - self.0.len() {
            return Err(fmt::Error);
        }
        self.0.push_str(s);
        Ok(())
    }
}

pub fn pong(buffer: &mut String, arg: Option<&str>) -> fmt::Result {
    buffer.clear();
    match arg {
        Some(arg) => write!(NoAllocWrite(buffer), "PONG :{}\r\n", arg),
        None => write!(NoAllocWrite(buffer), "PONG\r\n"),
    }
}
pub fn cap(buffer: &mut String, with_membership: bool) -> fmt::Result {
    buffer.clear();
    write!(
        NoAllocWrite(buffer),
        "CAP REQ :twitch.tv/commands twitch.tv/tags{}\r\n",
        if with_membership {
            " twitch.tv/membership"
        } else {
            ""
        }
    )
}
pub fn pass(buffer: &mut String, token: &str) -> fmt::Result {
    buffer.clear();
    write!(NoAllocWrite(buffer), "PASS {}\r\n", token)
}
pub fn nick(buffer: &mut String, login: &str) -> fmt::Result {
    buffer.clear();
    write!(NoAllocWrite(buffer), "NICK {}\r\n", login)
}
pub fn join(buffer: &mut String, channel: &str) -> fmt::Result {
    buffer.clear();
    write!(NoAllocWrite(buffer), "JOIN #{}\r\n", channel)
}
pub fn part(buffer: &mut String, channel: &str) -> fmt::Result {
    buffer.clear();
    write!(NoAllocWrite(buffer), "PART #{}\r\n", channel)
}

pub fn privmsg(
    buffer: &mut String,
    channel: &str,
    smb: &mut SameMessageBypass,
    message: &str,
) -> fmt::Result {
    buffer.clear();
    write!(
        NoAllocWrite(buffer),
        "PRIVMSG #{} :{}{}\r\n",
        channel,
        message,
        smb.get()
    )
}
pub fn whisper(buffer: &mut String, user: &str, message: &str) -> fmt::Result {
    buffer.clear();
    // sending to special '#jtv' channel which is join-less, so messages can be
    // sent without any
    write!(
        NoAllocWrite(buffer),
        "PRIVMSG #jtv :/w {} {}\r\n",
        user,
        message
    )
}
pub fn me(buffer: &mut String, channel: &str, message: &str) -> fmt::Result {
    buffer.clear();
    write!(
        NoAllocWrite(buffer),
        "PRIVMSG #{} :/me {}\r\n",
        channel,
        message
    )
}
pub fn clear(buffer: &mut String, channel: &str) -> fmt::Result {
    buffer.clear();
    write!(NoAllocWrite(buffer), "PRIVMSG #{} :/clear\r\n", channel)
}
/// Maximum timeout is 2 weeks. In case `duration` is `None`, default is 10
/// minutes.
pub fn timeout(
    buffer: &mut String,
    channel: &str,
    user: &str,
    duration: Option<Duration>,
) -> fmt::Result {
    buffer.clear();
    match duration {
        Some(duration) => write!(
            NoAllocWrite(buffer),
            "PRIVMSG #{} :/timeout {} {}\r\n",
            channel,
            user,
            duration.num_seconds()
        ),
        None => write!(
            NoAllocWrite(buffer),
            "PRIVMSG #{} :/timeout {}\r\n",
            channel,
            user
        ),
    }
}
pub fn untimeout(buffer: &mut String, channel: &str, user: &str) -> fmt::Result {
    buffer.clear();
    write!(
        NoAllocWrite(buffer),
        "PRIVMSG #{} :/untimeout {}\r\n",
        channel,
        user
    )
}
pub fn ban(buffer: &mut String, channel: &str, user: &str) -> fmt::Result {
    buffer.clear();
    write!(
        NoAllocWrite(buffer),
        "PRIVMSG #{} :/ban {}\r\n",
        channel,
        user
    )
}
pub fn unban(buffer: &mut String, channel: &str, user: &str) -> fmt::Result {
    buffer.clear();
    write!(
        NoAllocWrite(buffer),
        "PRIVMSG #{} :/unban {}\r\n",
        channel,
        user
    )
}
pub enum Mode {
    R9K,
    Subscribers,
    Emote,
    Slow(Option<i64>),
    Followers(Option<Duration>),
}
pub fn roomstate(buffer: &mut String, channel: &str, mode: Mode, state: bool) -> fmt::Result {
    buffer.clear();
    write!(NoAllocWrite(buffer), "PRIVMSG #{} :", channel)?;
    match mode {
        Mode::R9K => match state {
            true => write!(NoAllocWrite(buffer), "/uniquechat")?,
            false => write!(NoAllocWrite(buffer), "/uniquechatoff")?,
        },
        Mode::Subscribers => match state {
            true => write!(NoAllocWrite(buffer), "/subscribers")?,
            false => write!(NoAllocWrite(buffer), "/subscribersoff")?,
        },
        Mode::Emote => match state {
            true => write!(NoAllocWrite(buffer), "/emoteonly")?,
            false => write!(NoAllocWrite(buffer), "/emoteonlyoff")?,
        },
        Mode::Slow(seconds) => match state {
            true => match seconds {
                Some(seconds) => write!(NoAllocWrite(buffer), "/slow {}", seconds)?,
                None => write!(NoAllocWrite(buffer), "/slow")?,
            },
            false => write!(NoAllocWrite(buffer), "/slowoff")?,
        },
        Mode::Followers(duration) => match state {
            true => match duration {
                Some(duration) => write!(
                    NoAllocWrite(buffer),
                    "/followers {}d {}s",
                    duration.num_days(),
                    duration.num_seconds() - (duration.num_days() * 86400i64)
                )?,
                None => write!(NoAllocWrite(buffer), "/followers")?,
            },
            false => write!(NoAllocWrite(buffer), "/followersoff")?,
        },
    }
    write!(NoAllocWrite(buffer), "\r\n")
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn write_output_is_correct() {
        let expected = "PRIVMSG #TEST :HELLO :)\r\n";
        let mut buf = String::with_capacity(expected.len());
        privmsg(
            &mut buf,
            "TEST",
            &mut SameMessageBypass::default(),
            "HELLO :)",
        )
        .unwrap();
        assert_eq!(buf, expected.to_string());
    }

    #[test]
    fn write_doesnt_allocate() {
        let mut buf = String::new();
        privmsg(&mut buf, "a", &mut SameMessageBypass::default(), "b").unwrap_err();
    }

    #[test]
    fn write_command_whisper() {
        // /w {USERNAME} {MESSAGE}
        let mut buf = String::with_capacity(1024);
        whisper(&mut buf, "USER", "MESSAGE").unwrap();
        assert_eq!(buf, "PRIVMSG #jtv :/w USER MESSAGE\r\n".to_string());
    }
    #[test]
    fn write_command_me() {
        // /me
        let mut buf = String::with_capacity(1024);
        me(&mut buf, "CHANNEL", "MESSAGE").unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/me MESSAGE\r\n".to_string());
    }
    #[test]
    fn write_command_clear() {
        // /clear
        let mut buf = String::with_capacity(1024);
        clear(&mut buf, "CHANNEL").unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/clear\r\n".to_string());
    }
    #[test]
    fn write_command_timeout() {
        // /timeout <user> [duration {SECONDS} = 600]
        let mut buf = String::with_capacity(1024);
        timeout(&mut buf, "CHANNEL", "USER", Some(Duration::weeks(2))).unwrap();
        assert_eq!(
            buf,
            "PRIVMSG #CHANNEL :/timeout USER 1209600\r\n".to_string()
        );
    }
    #[test]
    fn write_command_timeout_default() {
        // /timeout <user> [duration {SECONDS} = 600]
        let mut buf = String::with_capacity(1024);
        timeout(&mut buf, "CHANNEL", "USER", None).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/timeout USER\r\n".to_string());
    }
    #[test]
    fn write_command_untimeout() {
        // /untimeout <user>
        let mut buf = String::with_capacity(1024);
        untimeout(&mut buf, "CHANNEL", "USER").unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/untimeout USER\r\n".to_string());
    }
    #[test]
    fn write_command_ban() {
        // /ban <user>
        let mut buf = String::with_capacity(1024);
        ban(&mut buf, "CHANNEL", "USER").unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/ban USER\r\n".to_string());
    }
    #[test]
    fn write_command_unban() {
        // /unban <user>
        let mut buf = String::with_capacity(1024);
        unban(&mut buf, "CHANNEL", "USER").unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/unban USER\r\n".to_string());
    }
    #[test]
    fn write_command_uniquechat() {
        // /uniquechat
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::R9K, true).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/uniquechat\r\n".to_string());
    }
    #[test]
    fn write_command_uniquechatoff() {
        // /uniquechatoff
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::R9K, false).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/uniquechatoff\r\n".to_string());
    }
    #[test]
    fn write_command_subscribers() {
        // /subscribers
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Subscribers, true).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/subscribers\r\n".to_string());
    }
    #[test]
    fn write_command_subscribersoff() {
        // /subscribersoff
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Subscribers, false).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/subscribersoff\r\n".to_string());
    }
    #[test]
    fn write_command_emoteonly() {
        // /emoteonly
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Emote, true).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/emoteonly\r\n".to_string());
    }
    #[test]
    fn write_command_emoteonlyoff() {
        // /emoteonlyoff
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Emote, false).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/emoteonlyoff\r\n".to_string());
    }
    #[test]
    fn write_command_slow() {
        // /slow <{SECONDS}>
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Slow(Some(120)), true).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/slow 120\r\n".to_string());
    }
    #[test]
    fn write_command_slow_default() {
        // /slow <{SECONDS}>
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Slow(None), true).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/slow\r\n".to_string());
    }
    #[test]
    fn write_command_slowoff() {
        // /slowoff
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Slow(None), false).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/slowoff\r\n".to_string());
    }
    #[test]
    fn write_command_followers() {
        // /followers {TIME}
        let mut buf = String::with_capacity(1024);
        roomstate(
            &mut buf,
            "CHANNEL",
            Mode::Followers(Some(Duration::days(30) + Duration::seconds(120))),
            true,
        )
        .unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/followers 30d 120s\r\n".to_string());
    }
    #[test]
    fn write_command_default() {
        // /followers {TIME}
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Followers(None), true).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/followers\r\n".to_string());
    }
    #[test]
    fn write_command_followersoff() {
        // /followersoff
        let mut buf = String::with_capacity(1024);
        roomstate(&mut buf, "CHANNEL", Mode::Followers(None), false).unwrap();
        assert_eq!(buf, "PRIVMSG #CHANNEL :/followersoff\r\n".to_string());
    }
}
