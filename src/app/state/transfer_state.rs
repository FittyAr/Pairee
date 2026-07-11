use tokio::sync::mpsc;
use crate::fs::transfer::engine::TransferEngine;
use crate::fs::transfer::events::TransferEvent;
use crate::fs::transfer::job::{TransferProgress, TransferResults};

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
    Queue = 4,
}

pub struct TransferUIState {
    pub engine: TransferEngine,
    pub event_rx: mpsc::UnboundedReceiver<TransferEvent>,
    pub view_mode: TransferViewMode,
    pub active_tab: TransferTab,
    pub file_list_cursor: usize,
    pub file_list_scroll: usize,
    pub queue_cursor: usize,
    pub options_cursor: usize,
    
    // Snaphots de tiempo real para renderizar sin bloquear el hilo principal
    pub current_progress: Option<TransferProgress>,
    pub current_results: Option<crate::fs::transfer::job::TransferResults>,
    pub speed_info: (f64, Option<u64>), // (bytes_per_second, eta_seconds)
    pub log_lines: Vec<String>,
    pub post_action: crate::fs::transfer::post_action::PostAction,
    pub active_conflict_info: Option<(uuid::Uuid, std::path::PathBuf, crate::fs::transfer::conflict::ConflictInfo)>,
}

impl TransferUIState {
    pub fn new(engine: TransferEngine, event_rx: mpsc::UnboundedReceiver<TransferEvent>) -> Self {
        Self {
            engine,
            event_rx,
            view_mode: TransferViewMode::Hidden,
            active_tab: TransferTab::FileList,
            file_list_cursor: 0,
            file_list_scroll: 0,
            queue_cursor: 0,
            options_cursor: 0,
            current_progress: None,
            current_results: None,
            speed_info: (0.0, None),
            log_lines: Vec::new(),
            post_action: crate::fs::transfer::post_action::PostAction::None,
            active_conflict_info: None,
        }
    }
}
