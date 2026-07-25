use crate::fs::transfer::engine::TransferEngine;
use crate::fs::transfer::events::TransferEvent;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferViewMode {
    Hidden,
    Minimized,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferTab {
    FileList = 0,
    Options = 1,
    Status = 2,
    Log = 3,
}

pub struct TransferUIState {
    pub engine: TransferEngine,
    pub event_rx: mpsc::UnboundedReceiver<TransferEvent>,
    pub view_mode: TransferViewMode,
    pub active_tab: TransferTab,
    pub file_list_cursor: usize,
    pub queue_cursor: usize,
    pub options_cursor: usize,

    // Snaphots de tiempo real para renderizar sin bloquear el hilo principal
    pub speed_info: (f64, Option<u64>), // (bytes_per_second, eta_seconds)
    pub post_action: crate::fs::transfer::post_action::PostAction,
    pub active_conflict_info: Option<(
        uuid::Uuid,
        std::path::PathBuf,
        crate::fs::transfer::conflict::ConflictInfo,
    )>,
}

impl TransferUIState {
    pub fn new(engine: TransferEngine, event_rx: mpsc::UnboundedReceiver<TransferEvent>) -> Self {
        Self {
            engine,
            event_rx,
            view_mode: TransferViewMode::Hidden,
            active_tab: TransferTab::FileList,
            file_list_cursor: 0,
            queue_cursor: 0,
            options_cursor: 0,
            speed_info: (0.0, None),
            post_action: crate::fs::transfer::post_action::PostAction::None,
            active_conflict_info: None,
        }
    }
}
