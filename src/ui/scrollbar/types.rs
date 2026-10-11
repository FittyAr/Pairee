use ratatui::layout::Rect;
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
    MultiRenamePreview,
}

/// Geometry + identity for one scrollbar painted last frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbarHitTarget {
    /// The track (thumb drag and click).
    pub area: Rect,
    /// The scrolled content: the mouse wheel over it scrolls.
    pub wheel_area: Rect,
    pub content_len: usize,
    pub viewport_len: usize,
    pub offset: usize,
    pub id: ScrollTargetId,
}

/// Persistent drag state; the tracks themselves are recorded in the frame's
/// [`HitMap`](crate::ui::hit::HitMap).
#[derive(Debug, Default)]
pub struct ScrollbarUiState {
    pub interaction: ScrollBarInteraction,
}

impl ScrollbarUiState {
    /// Records `target` in the frame being painted.
    pub fn register(&self, target: ScrollbarHitTarget) {
        // A wheel-only target (bar turned off) has an empty track.
        let area = if target.area.is_empty() {
            target.wheel_area
        } else {
            target.wheel_area.union(target.area)
        };
        crate::ui::hit::register(area, crate::ui::hit::HitTarget::Scrollbar(target));
    }
}
