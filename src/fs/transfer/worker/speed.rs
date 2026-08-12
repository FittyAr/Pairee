use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::events::TransferEvent;

/// Spawns a background task that periodically reports transfer speed and ETA.
pub(super) fn spawn_speed_reporter(
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    job_id: Uuid,
    bytes_acc: Arc<AtomicU64>,
    is_cancelled: Arc<AtomicBool>,
    total_bytes: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_bytes = 0u64;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            if is_cancelled.load(Ordering::Relaxed) {
                break;
            }

            let current_bytes = bytes_acc.load(Ordering::SeqCst);
            let delta = current_bytes.saturating_sub(last_bytes);
            last_bytes = current_bytes;

            let bytes_per_second = delta as f64;
            let remaining_bytes = total_bytes.saturating_sub(current_bytes);
            let eta_seconds = if bytes_per_second > 0.0 {
                Some((remaining_bytes as f64 / bytes_per_second) as u64)
            } else {
                None
            };

            let _ = event_tx.send(TransferEvent::SpeedUpdate {
                job_id,
                bytes_per_second,
                eta_seconds,
            });

            if current_bytes >= total_bytes && total_bytes > 0 {
                break;
            }
        }
    })
}
