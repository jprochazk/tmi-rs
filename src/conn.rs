use std::{num::NonZeroU32, sync::Arc};

use chrono::Duration;
use futures::StreamExt;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    RateLimiter,
};
use thiserror::Error;
use tmi::write;
use tokio::{
    io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf},
    net::TcpStream,
};
use tokio_rustls::client::TlsStream;
use tokio_stream::wrappers::LinesStream;
pub use write::Mode;

use crate::{
    irc,
    tmi::{self, Message},
};

const TMI_URL_HOST: &str = "irc.chat.twitch.tv";
const TMI_TLS_PORT: u16 = 6697;

// TODO: rate limiting

#[derive(Clone, Debug, PartialEq)]
pub enum Login {
    Anonymous,
    Regular { login: String, token: String },
}

impl Default for Login {
    fn default() -> Self {
        Login::Anonymous
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Config {
    pub membership_data: bool,
    pub credentials: Login,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("Connection to Twitch IRC server failed")]
    ConnectionFailed,
    #[error("Encountered an I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Encountered an error while parsing: {0}")]
    Parse(#[from] tmi::parse::Error),
    #[error(transparent)]
    Generic(#[from] anyhow::Error),
    #[error("Timed out")]
    Timeout,
    #[error("Stream closed")]
    StreamClosed,
    #[error("Internal buffer is not large enough for message")]
    Formatting(#[from] std::fmt::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! err {
    ($Variant:ident, $msg:expr) => {
Err(err!(bare $Variant, $msg))
    };
    (bare $Variant:ident, $msg:expr) => {
        crate::conn::Error::$Variant(anyhow::anyhow!($msg))
    };
}

fn expected_cap_ack(request_membership_data: bool) -> &'static str {
    if request_membership_data {
        "twitch.tv/commands twitch.tv/tags twitch.tv/membership"
    } else {
        "twitch.tv/commands twitch.tv/tags"
    }
}

async fn connect_tls(host: &str, port: u16) -> Result<TlsStream<TcpStream>> {
    use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};

    let mut config = ClientConfig::new();
    config.root_store =
        rustls_native_certs::load_native_certs().expect("Failed to load native certs");
    let config = TlsConnector::from(Arc::new(config));
    let dnsname = DNSNameRef::try_from_ascii_str(host).map_err(|err| anyhow::anyhow!(err))?;
    let stream = TcpStream::connect((host, port))
        .await
        .map_err(|err| anyhow::anyhow!(err))?;
    let out = config
        .connect(dnsname, stream)
        .await
        .map_err(|err| anyhow::anyhow!(err))?;

    Ok(out)
}

pub struct Reader {
    stream: LinesStream<BufReader<ReadHalf<TlsStream<TcpStream>>>>,
}
impl Reader {
    pub fn new(stream: LinesStream<BufReader<ReadHalf<TlsStream<TcpStream>>>>) -> Reader {
        Reader { stream }
    }
    pub async fn next(&mut self) -> Result<Message> {
        if let Some(message) = self.stream.next().await {
            let message = message?;
            log::debug!("{}", message);
            Ok(Message::parse(message)?)
        } else {
            Err(Error::StreamClosed)
        }
    }
}

// TODO: Better rate-limiting
// rate limits are:
// * absolute (20msg / 30s)
// * per-channel (based on userstate + roomstate)
//   * VIP/Mod = can ignore
//   * Regular = global slow mode + room slow mode

// for now, it's always 1 message per second, which won't work everywhere
pub struct Sender {
    buffer: String,
    rate: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    stream: WriteHalf<TlsStream<TcpStream>>,
    smb: write::SameMessageBypass,
}
impl Sender {
    pub fn new(stream: WriteHalf<TlsStream<TcpStream>>) -> Sender {
        Sender {
            buffer: String::with_capacity(2048),
            rate: RateLimiter::direct(governor::Quota::per_second(NonZeroU32::new(1).unwrap())),
            stream,
            smb: write::SameMessageBypass::default(),
        }
    }
    /// Sends a raw `message` to twitch.
    ///
    /// `message` must be terminated with `\r\n`.
    ///
    /// Use at your own risk.
    pub async fn send(&mut self, message: &str) -> Result<()> {
        log::debug!("Sent message: {}", message.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(message.as_bytes()).await?;
        Ok(())
    }
    pub async fn pong(&mut self, arg: Option<&str>) -> Result<()> {
        write::pong(&mut self.buffer, arg)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Sends a capability request
    pub async fn cap(&mut self, with_membership: bool) -> Result<()> {
        write::cap(&mut self.buffer, with_membership)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Sends a `PASS oauth:<token>` message
    pub async fn pass(&mut self, token: &str) -> Result<()> {
        write::pass(&mut self.buffer, token)?;
        log::debug!("Sent message: PASS oauth:<...>");
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Sends a `NICK <login>` message
    pub async fn nick(&mut self, login: &str) -> Result<()> {
        write::nick(&mut self.buffer, login)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Join `channel`
    pub async fn join(&mut self, channel: &str) -> Result<()> {
        write::join(&mut self.buffer, channel)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Leave `channel`
    pub async fn part(&mut self, channel: &str) -> Result<()> {
        write::part(&mut self.buffer, channel)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Sends `message` to `channel`
    pub async fn privmsg(&mut self, channel: &str, message: &str) -> Result<()> {
        write::privmsg(&mut self.buffer, channel, &mut self.smb, message)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Send `message` directly to `user`
    pub async fn whisper(&mut self, user: &str, message: &str) -> Result<()> {
        write::whisper(&mut self.buffer, user, message)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Equivalent to `/me <message>`
    pub async fn me(&mut self, channel: &str, message: &str) -> Result<()> {
        write::whisper(&mut self.buffer, channel, message)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Clears chat in `channel`
    pub async fn clear(&mut self, channel: &str) -> Result<()> {
        write::clear(&mut self.buffer, channel)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Timeout `user` in `channel` for `duration`
    ///
    /// Maximum timeout is 2 weeks. In case `duration` is `None`, default is 10
    /// minutes.
    pub async fn timeout(
        &mut self,
        channel: &str,
        user: &str,
        duration: Option<Duration>,
    ) -> Result<()> {
        write::timeout(&mut self.buffer, channel, user, duration)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Removes `user`'s timeout in `channel`
    pub async fn untimeout(&mut self, channel: &str, user: &str) -> Result<()> {
        write::untimeout(&mut self.buffer, channel, user)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Ban `user` in `channel`
    pub async fn ban(&mut self, channel: &str, user: &str) -> Result<()> {
        write::ban(&mut self.buffer, channel, user)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// Unban `user` in `channel`
    pub async fn unban(&mut self, channel: &str, user: &str) -> Result<()> {
        write::unban(&mut self.buffer, channel, user)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
    /// For changing the room state, e.g. slow mode, emote-only mode, etc.
    pub async fn roomstate(&mut self, channel: &str, mode: Mode, state: bool) -> Result<()> {
        write::roomstate(&mut self.buffer, channel, mode, state)?;
        log::debug!("Sent message: {}", self.buffer.trim_end());
        self.rate.until_ready().await;
        self.stream.write_all(self.buffer.as_bytes()).await?;
        Ok(())
    }
}

pub struct Connection {
    pub sender: Sender,
    pub reader: Reader,
}

impl Connection {
    pub fn split(self) -> (Sender, Reader) {
        (self.sender, self.reader)
    }
    pub fn join(sender: Sender, reader: Reader) -> Connection {
        Connection { sender, reader }
    }
}

impl From<Connection> for (Sender, Reader) {
    fn from(value: Connection) -> (Sender, Reader) {
        value.split()
    }
}
impl From<(Sender, Reader)> for Connection {
    fn from(value: (Sender, Reader)) -> Connection {
        Connection::join(value.0, value.1)
    }
}

pub async fn connect(config: Config) -> Result<Connection> {
    log::debug!("Connecting to TMI");
    // 1. connect
    let connection: TlsStream<TcpStream> = tokio::time::timeout(
        Duration::seconds(5)
            .to_std()
            .expect("Failed to convert duration"),
        connect_tls(TMI_URL_HOST, TMI_TLS_PORT),
    )
    .await
    .or(Err(Error::Timeout))??;
    let (read, write) = split(connection);
    let mut read = LinesStream::new(BufReader::new(read).lines());
    let mut sender = Sender::new(write);

    // 2. request capabilities
    // < CAP REQ :twitch.tv/commands twitch.tv/tags [twitch.tv/membership]
    log::debug!(
        "Requesting capabilities: {}",
        if config.membership_data {
            "commands, tags, membership"
        } else {
            "commands, tags"
        }
    );

    sender.cap(config.membership_data).await?;
    // wait for CAP * ACK :twitch.tv/commands twitch.tv/tags [twitch.tv/membership]
    if let Some(line) = read.next().await {
        let line = line?;
        match tmi::Message::parse(line)? {
            tmi::Message::Capability(capability) => {
                if capability.which() != expected_cap_ack(config.membership_data) {
                    return err!(Generic, "Did not receive expected capabilities");
                }
            }
            _ => {
                return err!(Generic, "Did not receive expected capabilities");
            }
        }
    }

    // 3. authenticate
    match &config.credentials {
        Login::Anonymous => {
            let login = format!("justinfan{}", rand::thread_rng().gen_range(10000..99999));
            log::debug!("Authenticating as {}", login);
            use rand::Rng;
            // don't need PASS here
            sender.nick(&login).await?;
        }
        Login::Regular { login, token } => {
            log::debug!("Authenticating as {}", login);
            sender.pass(token).await?;
            sender.nick(login).await?;
        }
    }
    // wait for the '001' message, which means connection was successful
    if let Some(line) = read.next().await {
        let line = line?;
        match tmi::Message::parse(line)? {
            tmi::Message::Unknown(msg) => {
                if msg.cmd != irc::Command::Unknown("001".into()) {
                    return err!(Generic, "Failed to authenticate");
                }
            }
            _ => {
                return err!(Generic, "Failed to authenticate");
            }
        }
    }
    log::debug!("Connection successful");

    Ok(Connection::join(sender, Reader::new(read)))
}
