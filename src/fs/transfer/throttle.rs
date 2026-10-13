//! Bandwidth limit of a copy ("limit bandwidth rate"): the copier sleeps
//! whenever it gets ahead of the allowed average rate.

use std::time::{Duration, Instant};

pub struct Throttle {
    /// Bytes per second; `None` (or 0) means unlimited.
    rate: Option<u64>,
    began: Instant,
}

impl Throttle {
    pub fn new(rate: Option<u64>) -> Self {
        Self {
            rate: rate.filter(|&r| r > 0),
            began: Instant::now(),
        }
    }

    /// Sleeps (blocking) until `bytes` sent since the start fit the rate.
    pub fn pace(&self, bytes: u64) {
        if let Some(rate) = self.rate {
            let expected = Duration::from_secs_f64(bytes as f64 / rate as f64);
            if let Some(ahead) = expected.checked_sub(self.began.elapsed()) {
                std::thread::sleep(ahead);
            }
        }
    }
}
