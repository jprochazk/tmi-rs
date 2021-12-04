use super::util::UnsafeSlice;
use chrono::{DateTime, Duration, TimeZone, Utc};
use std::collections::HashMap;
use thiserror::Error;
use twitch_getters::twitch_getters;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("Expected tag '{0}'")]
    MissingTag(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[twitch_getters]
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub tags: Tags,
    pub prefix: Option<Prefix>,
    pub cmd: Command,
    channel: Option<UnsafeSlice>,
    pub params: Option<Params>,
    pub source: String,
}

// SAFETY: it is safe to send `UnsafeSlice` across threads, as long as they are sent together with their source String
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for Message {}

impl Message {
    /// Parse a raw IRC Message
    ///
    /// Parses some Twitch-specific things, such as
    /// nick-only prefixes being host-only, or
    /// the #<channel id> always being present
    /// before :params
    pub fn parse(source: String) -> Result<Message> {
        let (tags, remainder) = Tags::parse(source.trim());
        let (prefix, remainder) = Prefix::parse(remainder);
        let (cmd, remainder) = Command::parse(remainder);
        let (channel, remainder) = Channel::parse(remainder);
        let params = Params::parse(remainder);

        Ok(Message {
            tags,
            prefix,
            cmd,
            channel,
            params,
            source,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Ping,
    Pong,
    /// Join channel
    Join,
    /// Leave channel
    Part,
    /// Twitch Private Message
    Privmsg,
    // Twitch extensions
    /// Send message to a single user
    Whisper,
    /// Purge a user's messages
    Clearchat,
    /// Single message removal
    Clearmsg,
    /// Sent upon successful authentication (PASS/NICK command)
    GlobalUserState,
    /// Channel starts or stops host mode
    HostTarget,
    /// General notices from the server
    Notice,
    /// Rejoins channels after a restart
    Reconnect,
    /// Identifies the channel's chat settings
    RoomState,
    /// Announces Twitch-specific events to the channel
    UserNotice,
    /// Identifies a user's chat settings or properties
    UserState,
    /// Requesting an IRC capability
    Capability,
    /// Unknown command
    Unknown(String),
}

impl Command {
    /// Parses a Twitch IRC command
    ///
    /// Returns (command, remainder)
    fn parse(data: &str) -> (Command, &str) {
        use Command::*;
        let data = data.trim_start();
        let end = match data.find(' ') {
            Some(v) => v,
            None => data.len(),
        };
        let cmd = &data[..end];
        let cmd = match cmd {
            "PING" => Ping,
            "PONG" => Pong,
            "JOIN" => Join,
            "PART" => Part,
            "PRIVMSG" => Privmsg,
            "WHISPER" => Whisper,
            "CLEARCHAT" => Clearchat,
            "CLEARMSG" => Clearmsg,
            "GLOBALUSERSTATE" => GlobalUserState,
            "HOSTTARGET" => HostTarget,
            "NOTICE" => Notice,
            "RECONNECT" => Reconnect,
            "ROOMSTATE" => RoomState,
            "USERNOTICE" => UserNotice,
            "USERSTATE" => UserState,
            "CAP" => Capability,
            other => Unknown(other.into()),
        };

        (cmd, &data[end..])
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tags(HashMap<UnsafeSlice, UnsafeSlice>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DurationKind {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl Tags {
    /// Parses IRC tags in the form
    ///
    /// `@key0=[value0];key1=[value1];...;keyN-1=[valueN-1];keyN=[valueN] `
    ///
    /// `[value]`s are optional
    ///
    /// Returns (tags, remainder)
    fn parse(data: &str) -> (Tags, &str) {
        let data = match data.strip_prefix('@') {
            Some(v) => v,
            None => data,
        };
        let mut map: HashMap<UnsafeSlice, UnsafeSlice> = HashMap::new();
        let mut end = 0;

        let mut current_key: Option<&str> = None;
        let mut remainder = data;
        let mut local_i = 0;
        let mut previous_char = "";

        // TODO: this apparently isn't enough to parse unicode...
        for (i, c) in data.grapheme_indices(true) {
            match current_key {
                None => match c {
                    // when we parse ';', save key, and parse value
                    "=" => {
                        current_key = Some(&remainder[..local_i]);
                        // remainder is set without this '='
                        remainder = &remainder[(local_i + 1)..];
                        local_i = 0;
                    }
                    c => {
                        local_i += c.as_bytes().len();
                    }
                },
                Some(key) => match c {
                    // when we parse ';', save value, push it into map
                    // and then parse key
                    ";" => {
                        // TODO the error may be here, the `..local_i` could be wrong.
                        // have to investigate.
                        let value = &remainder[..local_i];
                        if !value.is_empty() {
                            map.insert(key.into(), value.into());
                        }
                        // remainder is set without this ';'
                        remainder = &remainder[(local_i + 1)..];
                        local_i = 0;
                        current_key = None;
                    }
                    // if we parse a ' :', that's the end of tags
                    ":" if previous_char == " " => {
                        let value = &remainder[..(local_i - 1)];
                        if !value.trim().is_empty() {
                            map.insert(key.into(), value.into());
                        }
                        end = i;
                        break;
                    }
                    c => {
                        local_i += c.as_bytes().len();
                    }
                },
            }
            previous_char = c;
        }

        (Tags(map), &data[end..])
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(&(key.into())).copied().map(|s| s.as_str())
    }

    /// Iterates the tags to find one with key == `key`.
    pub(crate) fn get_raw(&self, key: &str) -> Option<UnsafeSlice> {
        self.0.get(&(key.into())).copied()
    }

    /// Parses a string, transforming all whitespace "\\s" to actual whitespace.
    pub(crate) fn get_ns(&self, key: &str) -> Option<String> {
        self.get_raw(key).map(|v| {
            let v = v.as_ref();
            let mut out = String::with_capacity(v.len());
            let mut parts = v.split("\\s").peekable();
            while let Some(part) = parts.next() {
                out.push_str(part);
                if parts.peek().is_some() {
                    out.push(' ');
                }
            }
            out
        })
    }

    /// Parses a number
    pub(crate) fn get_number<N>(&self, key: &str) -> Option<N>
    where
        N: std::str::FromStr,
        <N as std::str::FromStr>::Err: std::fmt::Display,
    {
        match self.get_raw(key) {
            Some(v) => match v.as_ref().parse::<N>() {
                Ok(v) => Some(v),
                Err(_) => None,
            },
            None => None,
        }
    }

    /// Parses a numeric bool (0 or 1)
    pub(crate) fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get_raw(key) {
            Some(v) => match v.as_ref() {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            },
            None => None,
        }
    }

    // Parses a comma-separated list of values
    /* pub(crate) fn get_csv(&self, key: &str) -> Option<Vec<UnsafeSlice>> {
        self.get_raw(key).map(|v| {
            v.as_ref()
                .split(',')
                .filter(|v| !v.is_empty())
                .map(|v| v.into())
                .collect()
        })
    } */

    /// Parses a millisecond precision UNIX timestamp as a UTC date/time
    pub(crate) fn get_date(&self, key: &str) -> Option<DateTime<Utc>> {
        self.get_number::<i64>(key).map(|v| Utc.timestamp_millis(v))
    }

    pub(crate) fn get_duration(&self, key: &str, kind: DurationKind) -> Option<Duration> {
        match self.get_number::<i64>(key) {
            Some(v) => match kind {
                DurationKind::Nanoseconds => Some(Duration::nanoseconds(v)),
                DurationKind::Microseconds => Some(Duration::microseconds(v)),
                DurationKind::Milliseconds => Some(Duration::milliseconds(v)),
                DurationKind::Seconds => Some(Duration::seconds(v)),
                DurationKind::Minutes => Some(Duration::minutes(v)),
                DurationKind::Hours => Some(Duration::hours(v)),
                DurationKind::Days => Some(Duration::days(v)),
                DurationKind::Weeks => Some(Duration::weeks(v)),
            },
            None => None,
        }
    }

    pub fn require(&self, key: &str) -> Result<&str> {
        self.get_raw(key)
            .map(|v| v.as_str())
            .ok_or_else(|| Error::MissingTag(key.into()))
    }

    /// Like `.get()`, but returns an `Error` in case the key doesn't exist,
    /// or is invalid in some way
    pub(crate) fn require_raw(&self, key: &str) -> Result<UnsafeSlice> {
        self.get_raw(key)
            .ok_or_else(|| Error::MissingTag(key.into()))
    }

    /// Like `.get_ns()`, but returns an `Error` in case the key doesn't exist,
    /// or is invalid in some way
    pub(crate) fn require_ns(&self, key: &str) -> Result<String> {
        self.get_ns(key)
            .ok_or_else(|| Error::MissingTag(key.into()))
    }

    /// Like `.get_number()`, but returns an `Error` in case the key doesn't
    /// exist, or is invalid in some way
    pub(crate) fn require_number<N>(&self, key: &str) -> Result<N>
    where
        N: std::str::FromStr,
        <N as std::str::FromStr>::Err: std::fmt::Display,
    {
        self.get_number(key)
            .ok_or_else(|| Error::MissingTag(key.into()))
    }

    // Like `.get_bool()`, but returns an `Error` in case the key doesn't
    // exist, or is invalid in some way
    /* pub(crate) fn require_bool(&self, key: &str) -> Result<bool> {
        self.get_bool(key)
            .ok_or_else(|| Error::MissingTag(key.into()))
    } */

    // Like `.get_csv()`, but returns an `Error` in case the key doesn't exist,
    // or is invalid in some way
    /* pub(crate) fn require_csv(&self, key: &str) -> Result<Vec<UnsafeSlice>> {
        self.get_csv(key)
            .ok_or_else(|| Error::MissingTag(key.into()))
    } */

    /// Like `.get_date()`, but returns an `Error` in case the key doesn't
    /// exist, or is invalid in some way
    pub(crate) fn require_date(&self, key: &str) -> Result<DateTime<Utc>> {
        self.get_date(key)
            .ok_or_else(|| Error::MissingTag(key.into()))
    }

    // Like `.get_duration()`, but returns an `Error` in case the key doesn't
    // exist, or is invalid in some way
    /* pub(crate) fn require_duration(&self, key: &str, kind: DurationKind) -> Result<Duration> {
        self.get_duration(key, kind)
            .ok_or_else(|| Error::MissingTag(key.into()))
    } */
}

#[twitch_getters]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prefix {
    nick: Option<UnsafeSlice>,
    user: Option<UnsafeSlice>,
    host: UnsafeSlice,
}

impl Prefix {
    /// Parses an IRC prefix in one of the following forms:
    ///
    /// * `host`
    /// * `nick@host`
    /// * `nick!user@host`
    ///
    /// Returns (prefix, remainder)
    fn parse(data: &str) -> (Option<Prefix>, &str) {
        let data = data.trim_start();
        if data.starts_with(':') {
            let start = 0;
            let end = match data[start..].find(' ') {
                Some(end) => start + end,
                None => start + data[start..].len(),
            };

            let prefix = &data[start + 1..end];

            // on twitch, nick-only is actually host-only (because they're not fully
            // compliant with RFC2812) so in case we don't find '@', we treat
            // the prefix as just the 'host' part
            let (nick, user, host) = match prefix.split_once('@') {
                Some((nick_and_user, host)) => match nick_and_user.split_once('!') {
                    // case: 'nick!user@host'
                    Some((nick, user)) => (Some(nick), Some(user), host),
                    // case: 'nick@host'
                    None => (Some(nick_and_user), None, host),
                },
                // case: 'host'
                None => (None, None, prefix),
            };

            (
                Some(Prefix {
                    nick: nick.map(|v| v.into()),
                    user: user.map(|v| v.into()),
                    host: host.into(),
                }),
                &data[end..],
            )
        } else {
            (None, data)
        }
    }
}

pub struct Channel;
impl Channel {
    fn parse(data: &str) -> (Option<UnsafeSlice>, &str) {
        let data = data.trim_start();
        let (mut start, mut end) = (None, data.len());
        for (i, c) in data.char_indices() {
            match c {
                // No channel, because we found the start of :message
                // TODO: write test that takes into account '#' being present in the message
                ':' if start.is_none() => {
                    return (None, data);
                }
                // Either we found `end`
                ' ' if start.is_some() => {
                    end = i;
                    break;
                }
                // or nothing
                ' ' => {
                    return (None, data);
                }
                // We found `start`
                '#' => start = Some(i),
                _ => (),
            }
        }
        let (start, end) = match (start, end) {
            (Some(s), e) => (s, e),
            _ => return (None, data),
        };
        let (channel, remainder) = data[start..].split_at(end);
        (Some((&channel[1..]).into()), remainder)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params(UnsafeSlice);
impl Params {
    /// Parse a params list
    ///
    /// Valid form: `[:]param0 [:]param1 [:]param2 [:]param3"
    fn parse(data: &str) -> Option<Params> {
        let data = data.trim_start();
        if data.is_empty() {
            None
        } else {
            Some(Params(data.into()))
        }
    }

    pub(crate) fn raw(&self) -> &str {
        self.0.as_str()
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0
            .as_str()
            .split(' ')
            .map(|v| v.strip_prefix(':').unwrap_or(v))
            .filter(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn parse_empty_prefix() {
        assert_eq!(None, Prefix::parse("PING :tmi.twitch.tv").0)
    }

    #[test]
    fn parse_prefix_host_only() {
        // :test.tmi.twitch.tv
        assert_eq!(
            Some(Prefix {
                nick: None,
                user: None,
                host: "tmi.twitch.tv".into()
            }),
            Prefix::parse(":tmi.twitch.tv").0
        );
    }

    #[test]
    fn parse_prefix_host_and_nick() {
        // :test@test.tmi.twitch.tv
        assert_eq!(
            Some(Prefix {
                nick: Some("test".into()),
                user: None,
                host: "test.tmi.twitch.tv".into()
            }),
            Prefix::parse(":test@test.tmi.twitch.tv").0
        );
    }

    #[test]
    fn parse_prefix_full() {
        // :test!test@test.tmi.twitch.tv
        assert_eq!(
            Some(Prefix {
                nick: Some("test".into()),
                user: Some("test".into()),
                host: "test.tmi.twitch.tv".into()
            }),
            Prefix::parse(":test!test@test.tmi.twitch.tv").0
        );
    }

    #[test]
    fn parse_command() {
        assert_eq!(Command::Privmsg, Command::parse("PRIVMSG").0)
    }

    // TODO: tests for parsing other message types

    #[test]
    fn parse_real_ping() {
        let src = "PING :tmi.twitch.tv".to_string();
        assert_eq!(
            Message {
                tags: Tags(HashMap::new()),
                prefix: None,
                cmd: Command::Ping,
                channel: None,
                params: Some(Params(":tmi.twitch.tv".into())),
                source: src.clone()
            },
            Message::parse(src).unwrap()
        )
    }

    #[test]
    fn parse_join() {
        let src = ":test!test@test.tmi.twitch.tv JOIN #channel".to_string();

        assert_eq!(
            Message {
                tags: Tags(HashMap::new()),
                prefix: Some(Prefix {
                    nick: Some("test".into()),
                    user: Some("test".into()),
                    host: "test.tmi.twitch.tv".into()
                }),
                cmd: Command::Join,
                channel: Some("channel".into()),
                params: None,
                source: src.clone()
            },
            Message::parse(src).unwrap()
        )
    }

    #[test]
    fn parse_full_privmsg() {
        let src = "\
            @badge-info=;\
            badges=;\
            color=#0000FF;\
            display-name=JuN1oRRRR;\
            emotes=;\
            flags=;\
            id=e9d998c3-36f1-430f-89ec-6b887c28af36;\
            mod=0;\
            room-id=11148817;\
            subscriber=0;\
            tmi-sent-ts=1594545155039;\
            turbo=0;\
            user-id=29803735;\
            user-type= \
            :jun1orrrr!jun1orrrr@jun1orrrr.tmi.twitch.tv PRIVMSG #pajlada :dank cam\
        "
        .to_string();
        assert_eq!(
            Message {
                tags: Tags(
                    vec![
                        ("color", "#0000FF"),
                        ("display-name", "JuN1oRRRR"),
                        ("id", "e9d998c3-36f1-430f-89ec-6b887c28af36"),
                        ("mod", "0"),
                        ("room-id", "11148817"),
                        ("subscriber", "0"),
                        ("tmi-sent-ts", "1594545155039"),
                        ("turbo", "0"),
                        ("user-id", "29803735"),
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect()
                ),
                prefix: Some(Prefix {
                    nick: Some("jun1orrrr".into()),
                    user: Some("jun1orrrr".into()),
                    host: "jun1orrrr.tmi.twitch.tv".into()
                }),
                cmd: Command::Privmsg,
                channel: Some("pajlada".into()),
                params: Some(Params(":dank cam".into())),
                source: src.clone()
            },
            Message::parse(src).unwrap()
        );
    }

    #[test]
    fn parse_whisper_with_emotes() {
        let src = "\
        @badges=;color=#2E8B57;display-name=pajbot ;emotes=25:7-11;message-id=\
        2034;thread-id=40286300_82008718;turbo=0;user-id=82008718;user-type= \
        :pajbot!pajbot@pajbot.tmi.twitch.tv WHISPER randers :Riftey Kappa\
        "
        .to_string();
        assert_eq!(
            Message {
                tags: Tags(
                    vec![
                        ("message-id", "2034"),
                        ("emotes", "25:7-11"),
                        ("turbo", "0"),
                        ("thread-id", "40286300_82008718"),
                        ("user-id", "82008718"),
                        ("color", "#2E8B57"),
                        ("display-name", "pajbot "),
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
                ),
                prefix: Some(Prefix {
                    nick: Some("pajbot".into()),
                    user: Some("pajbot".into()),
                    host: "pajbot.tmi.twitch.tv".into(),
                }),
                cmd: Command::Whisper,
                channel: None,
                params: Some(Params("randers :Riftey Kappa".into())),
                source: src.clone(),
            },
            Message::parse(src).unwrap()
        );
    }

    #[test]
    fn parse_whisper_with_action() {
        let src = "\
        @badges=;color=#2E8B57;display-name=pajbot;emotes=25:7-11;message-id=\
        2034;thread-id=40286300_82008718;turbo=0;user-id=82008718;user-type= \
        :pajbot!pajbot@pajbot.tmi.twitch.tv WHISPER randers :\x01ACTION Riftey Kappa\x01\
        "
        .to_string();
        assert_eq!(
            Message {
                tags: Tags(
                    vec![
                        ("message-id", "2034"),
                        ("emotes", "25:7-11"),
                        ("turbo", "0"),
                        ("thread-id", "40286300_82008718"),
                        ("user-id", "82008718"),
                        ("color", "#2E8B57"),
                        ("display-name", "pajbot"),
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
                ),
                prefix: Some(Prefix {
                    nick: Some("pajbot".into()),
                    user: Some("pajbot".into()),
                    host: "pajbot.tmi.twitch.tv".into(),
                }),
                cmd: Command::Whisper,
                channel: None,
                params: Some(Params("randers :\x01ACTION Riftey Kappa\x01".into())),
                source: src.clone(),
            },
            Message::parse(src).unwrap()
        );
    }

    #[test]
    fn parse_msg_with_extra_semicolons() {
        let src = "\
        @login=supibot;room-id=;target-msg-id=25fd76d9-4731-4907-978e-a391134ebd67;\
        tmi-sent-ts=-6795364578871 :tmi.twitch.tv CLEARMSG #randers :Pong! Uptime: 6h,\
        15m; Temperature: 54.8°C; Latency to TMI: 183ms; Commands used: 795\
        "
        .to_string();
        assert_eq!(
            Message {
                tags: Tags(
                    vec![
                        ("login", "supibot"),
                        ("target-msg-id", "25fd76d9-4731-4907-978e-a391134ebd67"),
                        ("tmi-sent-ts", "-6795364578871")
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
                ),
                prefix: Some(Prefix {
                    nick: None,
                    user: None,
                    host: "tmi.twitch.tv".into(),
                }),
                cmd: Command::Clearmsg,
                channel: Some("randers".into()),
                params: Some(Params(
                    ":Pong! Uptime: 6h,15m; Temperature: 54.8°C; Latency to TMI: 183ms; Commands used: 795".into()
                )),
                source: src.clone(),
            },
            Message::parse(src).unwrap()
        )
    }

    #[test]
    fn parse_tags_with_long_unicode_chars() {
        // TODO
        let src = "\
        @login=supibot;room-id=;target-msg-id=25fd76d9-4731-4907-978e-a391134ebd67;tmi-sent-ts=-6795364578871;\
        some-tag=とりくしい :tmi.twitch.tv CLEARMSG #randers :asdf\
        "
        .to_string();
        assert_eq!(
            Message {
                tags: Tags(
                    vec![
                        ("login", "supibot"),
                        ("target-msg-id", "25fd76d9-4731-4907-978e-a391134ebd67"),
                        ("tmi-sent-ts", "-6795364578871"),
                        ("some-tag", "とりくしい")
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
                ),
                prefix: Some(Prefix {
                    nick: None,
                    user: None,
                    host: "tmi.twitch.tv".into(),
                }),
                cmd: Command::Clearmsg,
                channel: Some("randers".into()),
                params: Some(Params(":asdf".into())),
                source: src.clone(),
            },
            Message::parse(src).unwrap()
        )
    }
}
