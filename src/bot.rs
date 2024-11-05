use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::client::read::RecvError;
use crate::client::write::{SameMessageBypass, SendError};
use crate::client::{Auth, Config, ConnectError, ReconnectError};
use crate::common::JoinIter as _;
use crate::{Client, Message, MessageParseError, Privmsg};

fn now() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis()
}

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
pub struct Context {
  inner: mpsc::UnboundedSender<Command>,
  is_anon: bool,
}

static_assert_send!(Context);
static_assert_sync!(Context);

impl Context {
  pub fn is_anon(&self) -> bool {
    self.is_anon
  }

  pub fn join(&self, channel: impl Into<String>) {
    let channel = channel.into();
    self.inner.send(Command::Join { channel }).unwrap();
  }

  pub fn join_all(&self, channels: impl IntoIterator<Item = impl Into<String>>) {
    let channels = channels.into_iter().map(|c| c.into()).collect();
    self.inner.send(Command::JoinAll { channels }).unwrap();
  }

  pub fn part(&self, channel: impl Into<String>) {
    let channel = channel.into();
    self.inner.send(Command::Part { channel }).unwrap();
  }

  /// Create a message to send to the given channel.
  ///
  /// ```rust
  /// # async fn test(ctx: tmi::Context) {
  /// ctx.privmsg("#pajlada", "hey guys").send();
  /// # }
  /// ```
  pub fn privmsg(&self, channel: impl Into<String>, text: impl Into<String>) -> PrivmsgBuilder {
    let channel = channel.into();
    let text = text.into();
    PrivmsgBuilder {
      ctx: self,
      channel,
      text,
      reply_to: None,
    }
  }
}

pub struct PrivmsgBuilder<'a> {
  ctx: &'a Context,
  channel: String,
  text: String,
  reply_to: Option<String>,
}

impl<'a> PrivmsgBuilder<'a> {
  pub fn reply_to(mut self, id: impl Into<String>) -> Self {
    self.reply_to = Some(id.into());
    self
  }

  /// Send the message.
  ///
  /// If the sender is anonymous, this will do nothing.
  pub fn send(self) {
    if self.ctx.is_anon() {
      return;
    }

    self
      .ctx
      .inner
      .send(Command::Privmsg {
        channel: self.channel,
        text: self.text,
        reply_to: self.reply_to,
      })
      .unwrap();
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

  pub fn auth(mut self, auth: Option<impl Into<Auth>>) -> Self {
    self.config = self.config.auth(auth);
    self
  }

  pub fn channels(mut self, channels: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.channels = channels.into_iter().map(|v| v.into()).collect();
    self
  }

  pub async fn spawn<F, Fut>(self, handler: F) -> Result<Context, BotError>
  where
    F: Fn(Context, Privmsg<'static>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), BotError>> + Send + Sync,
  {
    let (sender, receiver) = mpsc::unbounded_channel();
    let ctx = Context {
      inner: sender,
      is_anon: self.config.auth.is_none(),
    };
    ctx.join_all(self.channels);

    let client = Client::connect(self.config).await?;
    tokio::spawn({
      let ctx = ctx.clone();
      async move {
        State::new(ctx, receiver, client)
          .run_in_place(handler)
          .await
      }
    });
    Ok(ctx)
  }

  pub async fn run_in_place<F, Fut>(self, handler: F) -> Result<(), BotError>
  where
    F: Fn(Context, Privmsg<'static>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), BotError>> + Send + Sync,
  {
    let (sender, receiver) = mpsc::unbounded_channel();
    let ctx = Context {
      inner: sender,
      is_anon: self.config.auth.is_none(),
    };
    ctx.join_all(self.channels);

    let client = Client::connect(self.config).await?;
    State::new(ctx, receiver, client)
      .run_in_place(handler)
      .await
  }
}

impl Default for Bot {
  fn default() -> Self {
    Self::new()
  }
}

pub async fn spawn<F, Fut>(
  channels: impl IntoIterator<Item = impl Into<String>>,
  handler: F,
) -> Result<Context, BotError>
where
  F: Fn(Context, Privmsg<'static>) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = Result<(), BotError>> + Send + Sync,
{
  Bot::new().channels(channels).spawn(handler).await
}

pub async fn run_in_place<F, Fut>(
  channels: impl IntoIterator<Item = impl Into<String>>,
  handler: F,
) -> Result<(), BotError>
where
  F: Fn(Context, Privmsg<'static>) -> Fut + Send + Sync,
  Fut: Future<Output = Result<(), BotError>> + Send + Sync,
{
  Bot::new().channels(channels).run_in_place(handler).await
}

struct State {
  ctx: Context,
  receiver: mpsc::UnboundedReceiver<Command>,
  client: Client,
  channels: HashMap<String, SameMessageBypass>,
}

impl State {
  fn new(ctx: Context, receiver: mpsc::UnboundedReceiver<Command>, client: Client) -> Self {
    Self {
      ctx,
      receiver,
      client,
      channels: HashMap::new(),
    }
  }

  async fn run_in_place<T: Handler>(mut self, handler: T) -> Result<(), BotError> {
    self.on_connect().await?;

    let mut ping_interval = tokio::time::interval(Duration::from_secs(60));

    loop {
      tokio::select! {
        _ = tokio::signal::ctrl_c() => {
          break;
        }
        _ = ping_interval.tick() => {
          let now = now().to_string();
          self.client.ping(&now).await?;
          trace!("send PING {now}");
        }
        msg = self.client.recv() => {
          let msg = msg?;
          let msg = msg.as_typed()?;
          self.handle_message(msg, &handler).await?;
        }
        cmd = self.receiver.recv() => {
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
    if self.client.config().auth.is_some() {
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
      Message::Privmsg(msg) => handler.handle(self.ctx.clone(), msg.into_owned()).await,
      Message::Ping(ping) => {
        trace!("recv PING");
        self.client.pong(&ping).await?;
        Ok(())
      }
      Message::Pong(pong) => {
        let nonce = pong.nonce().unwrap_or("");
        trace!("recv PONG {nonce}");
        if let Ok(nonce) = nonce.parse::<u128>() {
          trace!("latency: {}ms", now() - nonce);
        }
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

pub trait Handler {
  fn handle(
    &self,
    ctx: Context,
    msg: Privmsg<'static>,
  ) -> impl Future<Output = Result<(), BotError>> + Send;
}

impl<F, Fut> Handler for F
where
  F: Fn(Context, Privmsg<'static>) -> Fut + Send + Sync,
  Fut: Future<Output = Result<(), BotError>> + Send + Sync,
{
  async fn handle(&self, ctx: Context, msg: Privmsg<'static>) -> Result<(), BotError> {
    self(ctx, msg).await
  }
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
      BotError::Send(err) => write!(f, "{err}"),
      BotError::Recv(err) => write!(f, "{err}"),
      BotError::Parse(err) => write!(f, "{err}"),
      BotError::Connect(err) => write!(f, "{err}"),
      BotError::Reconnect(err) => write!(f, "{err}"),
    }
  }
}
