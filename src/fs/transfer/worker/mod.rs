//! Transfer worker: orchestrates scan → delete or copy/move phases.
//!
//! Public API is stable at this module root:
//! - [`TransferWorker`]
//! - [`is_destination_parent_dir`]

mod copy_phase;
mod delete_phase;
mod destination;
mod fs_helpers;
mod scan;
mod speed;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::events::TransferEvent;
use super::job::{TransferOperation, TransferResults};
use super::options::TransferOptions;

pub use destination::is_destination_parent_dir;

pub struct TransferWorker {
    pub job_id: Uuid,
    pub operation: TransferOperation,
    pub sources: Vec<PathBuf>,
    pub destination: PathBuf,
    pub options: TransferOptions,
    pub is_paused: Arc<AtomicBool>,
    pub is_cancelled: Arc<AtomicBool>,
    pub skip_file_flag: Arc<AtomicBool>,
    pub event_tx: mpsc::UnboundedSender<TransferEvent>,
    pub active_conflict:
        Arc<std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>>,
}

impl TransferWorker {
    pub fn new(
        job_id: Uuid,
        operation: TransferOperation,
        sources: Vec<PathBuf>,
        destination: PathBuf,
        options: TransferOptions,
        is_paused: Arc<AtomicBool>,
        is_cancelled: Arc<AtomicBool>,
        skip_file_flag: Arc<AtomicBool>,
        event_tx: mpsc::UnboundedSender<TransferEvent>,
        active_conflict: Arc<
            std::sync::Mutex<Option<crate::fs::transfer::conflict::ConflictResolution>>,
        >,
    ) -> Self {
        Self {
            job_id,
            operation,
            sources,
            destination,
            options,
            is_paused,
            is_cancelled,
            skip_file_flag,
            event_tx,
            active_conflict,
        }
    }

    pub async fn run(self) -> Result<TransferResults, anyhow::Error> {
        let _ = self.event_tx.send(TransferEvent::JobStarted {
            job_id: self.job_id,
        });

        // Detección LAN y optimización de buffers
        let is_lan = super::network::is_lan_path(&self.destination);
        let mut options = self.options.clone();
        if is_lan {
            options.buffer_size = crate::fs::transfer::options::BufferSize::_4MB;
        }

        // --- FASE 1: ESCANEO ---
        let scan_outcome = scan::scan(
            &self.sources,
            &self.destination,
            self.operation,
            &options,
            self.job_id,
            &self.is_cancelled,
            &self.event_tx,
        )?;

        if self.operation == TransferOperation::Delete {
            return delete_phase::run_delete_phase(
                &self.sources,
                scan_outcome.mappings,
                scan_outcome.dirs_to_delete,
                scan_outcome.total_bytes,
                &options,
                self.job_id,
                Arc::clone(&self.is_paused),
                Arc::clone(&self.is_cancelled),
                Arc::clone(&self.skip_file_flag),
                self.event_tx,
            )
            .await;
        }

        // Verificar espacio libre en destino
        if let Ok(free_space) = super::network::get_free_space(&self.destination) {
            if free_space < scan_outcome.total_bytes {
                let _ = self.event_tx.send(TransferEvent::FileSkipped {
                    job_id: self.job_id,
                    file: self.destination.clone(),
                    reason: format!(
                        "Warning: Low disk space. Required: {}, Available: {}",
                        bytesize::ByteSize(scan_outcome.total_bytes),
                        bytesize::ByteSize(free_space)
                    ),
                });
            }
        }

        // --- FASE 2: TRANSFERENCIA ---
        copy_phase::run_copy_phase(
            self.operation,
            scan_outcome.mappings,
            scan_outcome.dirs_to_delete,
            scan_outcome.total_bytes,
            &options,
            self.job_id,
            Arc::clone(&self.is_paused),
            Arc::clone(&self.is_cancelled),
            Arc::clone(&self.skip_file_flag),
            self.event_tx,
            self.active_conflict,
        )
        .await
    }
}
