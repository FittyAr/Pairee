use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::super::control::JobControl;
use super::super::events::TransferEvent;

/// Spawns a background task that periodically reports transfer speed and ETA.
pub(super) fn spawn_speed_reporter(
    ctl: &JobControl,
    bytes_acc: Arc<AtomicU64>,
    total_bytes: u64,
) -> tokio::task::JoinHandle<()> {
    let ctl = ctl.clone();
    tokio::spawn(async move {
        let mut last_bytes = 0u64;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            if ctl.is_cancelled() {
                break;
            }

            let current_bytes = bytes_acc.load(Ordering::SeqCst);
            let delta = current_bytes.saturating_sub(last_bytes);
            last_bytes = current_bytes;

            let bytes_per_second = delta as f64;
            let remaining_bytes = total_bytes.saturating_sub(current_bytes);
            let eta_seconds = (bytes_per_second > 0.0)
                .then(|| (remaining_bytes as f64 / bytes_per_second) as u64);

            ctl.emit(TransferEvent::SpeedUpdate {
                job_id: ctl.job_id,
                bytes_per_second,
                eta_seconds,
            });

            if current_bytes >= total_bytes && total_bytes > 0 {
                break;
            }
        }
    })
}
