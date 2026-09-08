use ratatui::layout::Rect;
use std::cell::RefCell;
use tui_scrollbar::ScrollBarInteraction;

/// Which theme surface colors to use for track/thumb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarSurface {
    /// File panels and full-screen viewer/quickview.
    Panel,
    /// Popups, help, history, transfer, git.
    Popup,
}

/// Logical scroll region that can receive mouse drag/jump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollTargetId {
    HelpContent,
    About,
    UpdateNotes,
    Viewer,
    QuickView,
    PanelLeft,
    PanelRight,
    GitList,
    HistoryCommand,
    HistoryView,
    HistoryFolder,
    TransferJobs,
    TransferFiles,
    TransferLog,
    PluginSelect,
}

/// Geometry + identity for one scrollbar painted last frame.
#[derive(Debug, Clone)]
pub struct ScrollbarHitTarget {
    pub area: Rect,
    pub content_len: usize,
    pub viewport_len: usize,
    pub offset: usize,
    pub id: ScrollTargetId,
}

/// Persistent drag state + hit targets registered during the last paint.
#[derive(Debug, Default)]
pub struct ScrollbarUiState {
    pub interaction: ScrollBarInteraction,
    targets: RefCell<Vec<ScrollbarHitTarget>>,
}

impl ScrollbarUiState {
    pub fn clear_targets(&self) {
        self.targets.borrow_mut().clear();
    }

    pub fn register(&self, target: ScrollbarHitTarget) {
        self.targets.borrow_mut().push(target);
    }

    pub fn targets_snapshot(&self) -> Vec<ScrollbarHitTarget> {
        self.targets.borrow().clone()
    }
}
