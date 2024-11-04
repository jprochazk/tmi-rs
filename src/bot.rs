use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use crate::client::read::RecvError;
use crate::client::write::Privmsg as PrivmsgProxy;
use crate::client::write::SameMessageBypass;
use crate::client::write::SendError;
use crate::client::Config;
use crate::client::ConnectError;
use crate::client::ReconnectError;
use crate::common::JoinIter as _;
use crate::Client;
use crate::Message;
use crate::MessageParseError;
use crate::Privmsg;

enum Command {
  Join {
    channel: String,
  },
  JoinAll {
    channels: Vec<String>,
  },
  Part {
    channel: String,
  },
  Privmsg {
    /// Channel to send the message to
    channel: String,

    /// Message text
    text: String,

    reply_to: Option<String>,
  },
}

#[derive(Clone)]
pub struct Sender {
  tx: mpsc::Sender<Command>,
}

static_assert_send!(Sender);
static_assert_sync!(Sender);

impl Sender {
  pub fn join(&self, channel: impl Into<String>) {
    let channel = channel.into();
    self.tx.blocking_send(Command::Join { channel }).unwrap();
  }

  pub fn join_all(&self, channels: impl IntoIterator<Item = impl Into<String>>) {
    let channels = channels.into_iter().map(|c| c.into()).collect();
    self
      .tx
      .blocking_send(Command::JoinAll { channels })
      .unwrap();
  }

  pub fn part(&self, channel: impl Into<String>) {
    let channel = channel.into();
    self.tx.blocking_send(Command::Part { channel }).unwrap();
  }

  pub fn privmsg(&self, channel: impl Into<String>, text: impl Into<String>) -> PrivmsgBuilder {
    let channel = channel.into();
    let text = text.into();
    PrivmsgBuilder {
      sender: self,
      channel,
      text,
      reply_to: None,
    }
  }
}

pub struct PrivmsgBuilder<'a> {
  sender: &'a Sender,
  channel: String,
  text: String,
  reply_to: Option<String>,
}

impl<'a> PrivmsgBuilder<'a> {
  pub fn reply_to(mut self, id: impl Into<String>) -> Self {
    self.reply_to = Some(id.into());
    self
  }

  pub fn send(self) {
    self
      .sender
      .tx
      .blocking_send(Command::Privmsg {
        channel: self.channel,
        text: self.text,
        reply_to: self.reply_to,
      })
      .unwrap();
  }
}

pub struct SenderProxy<'a> {
  state: &'a mut State,
}

impl<'a> SenderProxy<'a> {
  pub async fn join(&mut self, channel: impl Into<String>) -> Result<(), BotError> {
    self.state.maybe_join(channel.into()).await
  }

  pub async fn join_all(
    &mut self,
    channels: impl IntoIterator<Item = impl Into<String>>,
  ) -> Result<(), BotError> {
    for channel in channels {
      self.state.maybe_join(channel.into()).await?;
    }
    Ok(())
  }

  pub async fn part(&mut self, channel: impl Into<String>) -> Result<(), BotError> {
    self.state.maybe_part(channel.into()).await
  }

  pub fn privmsg<'data, 'client>(
    &'client mut self,
    channel: &'data str,
    text: &'data str,
  ) -> PrivmsgProxy<'data, 'client> {
    self.state.client.privmsg(channel, text)
  }
}

pub struct Bot {
  config: Config,
  channels: Vec<String>,
}

impl Bot {
  pub fn new() -> Self {
    Self {
      config: Config::default(),
      channels: Vec::new(),
    }
  }

  pub fn token(mut self, token: impl Into<String>) -> Self {
    self.config.token = Some(token.into());
    self
  }

  pub fn channels(mut self, channels: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.channels = channels.into_iter().map(|v| v.into()).collect();
    self
  }

  pub async fn spawn<T>(self, handler: T) -> Result<Sender, BotError>
  where
    T: Handler + Send + Sync + 'static,
  {
    let (tx, rx) = mpsc::channel(128);
    let sender = Sender { tx };
    sender.join_all(self.channels);

    let client = Client::connect(self.config).await?;
    tokio::spawn(async move { State::new(rx, client).run_in_place(handler).await });
    Ok(sender)
  }

  pub async fn run_in_place<T: Handler>(self, handler: T) -> Result<(), BotError> {
    let (tx, rx) = mpsc::channel(1);
    let sender = Sender { tx };
    sender.join_all(self.channels);

    let client = Client::connect(self.config).await?;
    State::new(rx, client).run_in_place(handler).await
  }
}

impl Default for Bot {
  fn default() -> Self {
    Self::new()
  }
}

pub async fn spawn<T>(
  channels: impl IntoIterator<Item = impl Into<String>>,
  handler: T,
) -> Result<Sender, BotError>
where
  T: Handler + Send + Sync + 'static,
{
  Bot::new().channels(channels).spawn(handler).await
}

pub async fn run_in_place<T: Handler>(
  channels: impl IntoIterator<Item = impl Into<String>>,
  handler: T,
) -> Result<(), BotError> {
  Bot::new().channels(channels).run_in_place(handler).await
}

struct State {
  rx: Receiver<Command>,
  client: Client,
  channels: HashMap<String, SameMessageBypass>,
}

impl State {
  fn new(rx: Receiver<Command>, client: Client) -> Self {
    Self {
      rx,
      client,
      channels: HashMap::new(),
    }
  }

  async fn run_in_place<T: Handler>(mut self, handler: T) -> Result<(), BotError> {
    self.on_connect().await?;

    let mut ping_interval = tokio::time::interval(Duration::from_secs(60));

    loop {
      // `tokio::select` either `ctrl-c` or `client.recv()`
      tokio::select! {
        _ = tokio::signal::ctrl_c() => {
          break;
        }
        _ = ping_interval.tick() => {
          let now = Instant::now().elapsed().as_millis().to_string();
          self.client.ping(&now).await?;
          trace!("send PING {now}");
        }
        msg = self.client.recv() => {
          let msg = msg?;
          let msg = msg.as_typed()?;
          self.handle_message(msg, &handler).await?;
        }
        cmd = self.rx.recv() => {
          match cmd {
            Some(cmd) => self.handle_cmd(cmd).await?,
            None => break,
          }
        }
      }
    }

    Ok(())
  }

  async fn on_connect(&mut self) -> Result<(), BotError> {
    if self.client.config().token.is_some() {
      trace!("bot connected with token");
    } else {
      trace!("bot connected anonymously");
    }
    trace!("joining channels: {}", self.channels.keys().join(", "));
    self.client.join_all(self.channels.keys()).await?;
    Ok(())
  }

  async fn handle_message<T: Handler>(
    &mut self,
    msg: Message<'_>,
    handler: &T,
  ) -> Result<(), BotError> {
    match msg {
      Message::Privmsg(msg) => {
        let context = Context { state: self };
        handler.handle(context, msg).await
      }
      Message::Ping(ping) => {
        trace!("recv PING");
        self.client.pong(&ping).await?;
        Ok(())
      }
      Message::Pong(pong) => {
        trace!("recv PONG {}", pong.nonce().unwrap_or(""));
        Ok(())
      }
      Message::Reconnect => {
        trace!("twitch requested a reconnect");
        self.client.reconnect().await?;
        self.on_connect().await
      }
      _ => Ok(()),
    }
  }

  async fn handle_cmd(&mut self, cmd: Command) -> Result<(), BotError> {
    match cmd {
      Command::Join { channel } => self.maybe_join(channel).await,
      Command::JoinAll { channels } => {
        for channel in channels {
          self.maybe_join(channel).await?;
        }
        Ok(())
      }
      Command::Part { channel } => self.maybe_part(channel).await,
      Command::Privmsg {
        channel,
        mut text,
        reply_to,
      } => {
        let smb = if !self.channels.contains_key(&channel) {
          self.channels.entry(channel.clone()).or_default()
        } else {
          self.channels.get_mut(&channel).unwrap()
        };
        text.push_str(smb.get());

        let mut privmsg = self.client.privmsg(&channel, &text);
        if let Some(msg_id) = &reply_to {
          privmsg = privmsg.reply_to(msg_id);
        }
        privmsg.send().await?;
        Ok(())
      }
    }
  }

  async fn maybe_join(&mut self, channel: String) -> Result<(), BotError> {
    if self.channels.contains_key(&channel) {
      return Ok(());
    }
    self.client.join(&channel).await?;
    self.channels.insert(channel, SameMessageBypass::default());
    Ok(())
  }

  async fn maybe_part(&mut self, channel: String) -> Result<(), BotError> {
    if !self.channels.contains_key(&channel) {
      return Ok(());
    }
    self.client.part(&channel).await?;
    let _ = self.channels.remove(&channel);
    Ok(())
  }
}

pub struct Context<'a> {
  state: &'a mut State,
}

impl<'a> Context<'a> {
  pub fn config(&self) -> &Config {
    self.state.client.config()
  }

  pub fn sender(&mut self) -> SenderProxy<'_> {
    SenderProxy { state: self.state }
  }
}

pub trait Handler: private::Sealed {
  fn handle(
    &self,
    ctx: Context<'_>,
    msg: Privmsg<'_>,
  ) -> impl Future<Output = Result<(), BotError>> + Send;
}

mod private {
  pub trait Sealed {}
}

impl<F, Fut> Handler for F
where
  Fut: Future<Output = Result<(), BotError>> + Send + Sync,
  F: Fn(Context<'_>, Privmsg<'_>) -> Fut + Send + Sync,
{
  async fn handle(&self, ctx: Context<'_>, msg: Privmsg<'_>) -> Result<(), BotError> {
    self(ctx, msg).await
  }
}

impl<F, Fut> private::Sealed for F
where
  Fut: Future<Output = Result<(), BotError>> + Send + Sync,
  F: Fn(Context<'_>, Privmsg<'_>) -> Fut + Send + Sync,
{
}

#[derive(Debug)]
pub enum BotError {
  Send(SendError),
  Recv(RecvError),
  Parse(MessageParseError),
  Connect(ConnectError),
  Reconnect(ReconnectError),
}

impl From<SendError> for BotError {
  fn from(err: SendError) -> Self {
    BotError::Send(err)
  }
}

impl From<RecvError> for BotError {
  fn from(err: RecvError) -> Self {
    BotError::Recv(err)
  }
}

impl From<MessageParseError> for BotError {
  fn from(err: MessageParseError) -> Self {
    BotError::Parse(err)
  }
}

impl From<ConnectError> for BotError {
  fn from(err: ConnectError) -> Self {
    BotError::Connect(err)
  }
}

impl From<ReconnectError> for BotError {
  fn from(err: ReconnectError) -> Self {
    BotError::Reconnect(err)
  }
}

impl std::error::Error for BotError {}

impl std::fmt::Display for BotError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BotError::Send(err) => write!(f, "failed to send: {err}"),
      BotError::Recv(err) => write!(f, "failed to receive: {err}"),
      BotError::Parse(err) => write!(f, "failed to parse: {err}"),
      BotError::Connect(err) => write!(f, "failed to connect: {err}"),
      BotError::Reconnect(err) => write!(f, "failed to reconnect: {err}"),
    }
  }
}
