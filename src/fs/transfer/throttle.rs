//! Bandwidth limit of a copy ("limit bandwidth rate"): the copier sleeps
//! whenever it gets ahead of the allowed average rate.

use std::time::{Duration, Instant};

/// Longest single sleep while pacing (cancellation is checked between).
const PACE_STEP: Duration = Duration::from_millis(100);

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

    /// Sleeps (blocking) until `bytes` fit the allowed rate, in short steps
    /// so a cancelled job (`cancelled`) stops at once: at a low rate one
    /// buffer can be worth hours of waiting.
    pub fn pace(&self, bytes: u64, cancelled: &dyn Fn() -> bool) {
        let Some(rate) = self.rate else {
            return;
        };
        let expected = Duration::from_secs_f64(bytes as f64 / rate as f64);
        while let Some(ahead) = expected.checked_sub(self.began.elapsed()) {
            if ahead.is_zero() || cancelled() {
                return;
            }
            std::thread::sleep(ahead.min(PACE_STEP));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancelled_job_stops_waiting_at_once() {
        // One megabyte at one byte per second would wait for days.
        let throttle = Throttle::new(Some(1));
        let start = Instant::now();
        throttle.pace(1024 * 1024, &|| true);
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
