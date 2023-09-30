use std::future::Future;
use std::time::Duration;

pub trait Timeout: Sized {
  fn timeout(self, duration: Duration) -> tokio::time::Timeout<Self>;
}

impl<F> Timeout for F
where
  F: Future,
{
  fn timeout(self, duration: Duration) -> tokio::time::Timeout<Self> {
    tokio::time::timeout(duration, self)
  }
}
