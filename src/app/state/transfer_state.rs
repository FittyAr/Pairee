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
}

pub struct TransferUIState {
    pub engine: TransferEngine,
    pub event_rx: mpsc::UnboundedReceiver<TransferEvent>,
    pub view_mode: TransferViewMode,
    pub active_tab: TransferTab,
    pub file_list_cursor: usize,
    pub file_list_scroll: usize,
    pub queue_cursor: usize,
    
    // Snaphots de tiempo real para renderizar sin bloquear el hilo principal
    pub current_progress: Option<TransferProgress>,
    pub current_results: Option<TransferResults>,
    pub speed_info: (f64, Option<u64>), // (bytes_per_second, eta_seconds)
    pub log_lines: Vec<String>,
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
            current_progress: None,
            current_results: None,
            speed_info: (0.0, None),
            log_lines: Vec::new(),
        }
    }
}
