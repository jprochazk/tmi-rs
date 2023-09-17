use std::time::{Duration, Instant};

pub struct Bucket {
  last_refreshed_at: Instant,
  capacity: u64,
  period: Duration,
  tokens: u64,
}

impl Bucket {
  pub fn new(capacity: u64, period: Duration, now: Instant) -> Self {
    Self {
      last_refreshed_at: now,
      capacity,
      period,
      tokens: capacity,
    }
  }

  pub fn refresh(&mut self, now: Instant) {
    if self.last_refreshed_at > now {
      return;
    }

    if now - self.last_refreshed_at >= self.period {
      self.tokens = self.capacity;
      self.last_refreshed_at = now;
    }
  }

  pub fn get(&mut self) -> bool {
    let ok = self.tokens > 0;
    self.tokens.saturating_sub(1);
    ok
  }
}
