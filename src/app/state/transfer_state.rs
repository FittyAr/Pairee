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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::engine::TransferEngine;

    #[test]
    fn transfer_view_mode_default_is_hidden() {
        // A freshly-constructed engine has no jobs, so hiding the
        // panel is the only sensible default.
        let (engine, rx) = TransferEngine::new();
        let state = TransferUIState::new(engine, rx);
        assert_eq!(state.view_mode, TransferViewMode::Hidden);
    }

    #[test]
    fn transfer_tab_default_is_file_list() {
        // Tabs are 0-indexed: FileList = 0, Options = 1, Status = 2, Log = 3.
        let (engine, rx) = TransferEngine::new();
        let state = TransferUIState::new(engine, rx);
        assert_eq!(state.active_tab, TransferTab::FileList);
        assert_eq!(TransferTab::FileList as usize, 0);
        assert_eq!(TransferTab::Options as usize, 1);
        assert_eq!(TransferTab::Status as usize, 2);
        assert_eq!(TransferTab::Log as usize, 3);
    }

    #[test]
    fn cursors_start_at_zero() {
        let (engine, rx) = TransferEngine::new();
        let state = TransferUIState::new(engine, rx);
        assert_eq!(state.file_list_cursor, 0);
        assert_eq!(state.queue_cursor, 0);
        assert_eq!(state.options_cursor, 0);
    }

    #[test]
    fn speed_info_starts_idle() {
        let (engine, rx) = TransferEngine::new();
        let state = TransferUIState::new(engine, rx);
        assert_eq!(state.speed_info, (0.0, None));
    }

    #[test]
    fn post_action_starts_as_none() {
        let (engine, rx) = TransferEngine::new();
        let state = TransferUIState::new(engine, rx);
        assert_eq!(
            state.post_action,
            crate::fs::transfer::post_action::PostAction::None
        );
    }

    #[test]
    fn active_conflict_starts_as_none() {
        // No conflict is in flight at construction time.
        let (engine, rx) = TransferEngine::new();
        let state = TransferUIState::new(engine, rx);
        assert!(state.active_conflict_info.is_none());
    }

    #[test]
    fn transfer_view_mode_equality_is_value_based() {
        // The enum derives PartialEq — make sure the variant names
        // and ordering didn't drift.
        assert_eq!(TransferViewMode::Hidden, TransferViewMode::Hidden);
        assert_eq!(TransferViewMode::Minimized, TransferViewMode::Minimized);
        assert_eq!(TransferViewMode::Expanded, TransferViewMode::Expanded);
        assert_ne!(TransferViewMode::Hidden, TransferViewMode::Expanded);
    }
}
