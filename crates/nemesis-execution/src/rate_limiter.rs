use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    max_tokens: u64,
    refill_rate_per_sec: u64,
    tokens: AtomicU64,
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(max_tokens: u64, refill_rate_per_sec: u64) -> Self {
        Self {
            max_tokens,
            refill_rate_per_sec,
            tokens: AtomicU64::new(max_tokens),
            last_refill: Mutex::new(Instant::now()),
        }
    }

    pub async fn acquire(&self) {
        loop {
            {
                let mut last = self.last_refill.lock().await;
                let elapsed = last.elapsed();
                let new_tokens = elapsed.as_secs() * self.refill_rate_per_sec;
                if new_tokens > 0 {
                    let current = self.tokens.load(Ordering::Relaxed);
                    let refilled = (current + new_tokens).min(self.max_tokens);
                    self.tokens.store(refilled, Ordering::Relaxed);
                    *last = Instant::now();
                }
            }

            let current = self.tokens.load(Ordering::Relaxed);
            if current > 0
                && self
                    .tokens
                    .compare_exchange(current, current - 1, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
            {
                return;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
