use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Move cursor selection up
    MoveUp,
    /// Move cursor selection down
    MoveDown,
    /// Move cursor one page up
    PageUp,
    /// Move cursor one page down
    PageDown,
    /// Jump cursor to the top item
    GoToTop,
    /// Jump cursor to the bottom item
    GoToBottom,
    /// Switch focus to the other panel (Tab)
    ChangePanel,
    /// Select / tag the highlighted item for bulk operations (Insert / Space / v)
    SelectItem,
    /// Execute file action or enter subdirectory (Enter / l)
    Execute,
    /// Go to the parent directory (Backspace / h)
    GoParent,
    /// Open help dialog (F1)
    Help,
    /// Open user custom commands menu (F2)
    UserMenu,
    /// View file content using native viewer (F3)
    View,
    /// Edit file using external text editor (F4 / y)
    Edit,
    /// Copy selected items to the other panel's path (F5)
    Copy,
    /// Rename or Move selected items (F6)
    Move,
    /// Make directory (F7)
    MkDir,
    /// Delete selected items (F8 / d)
    Delete,
    /// Open top pulldown menu (F9)
    Menu,
    /// Quit application (F10)
    Quit,
    /// Toggle visibility of dotfiles/hidden files (Ctrl+H)
    ToggleHidden,
    /// Focus cursor on terminal Command Line Input (:)
    FocusCli,
    /// Unfocus CLI / clear search filters / close popup (Esc)
    Unfocus,
    /// Refresh the files in both panels (Ctrl+R / F5 in some layouts)
    Refresh,
    /// Swap positions of left and right panels (Ctrl+U)
    SwapPanels,
    /// Open drive selection menu for the left panel (Alt+F1)
    DriveSelectLeft,
    /// Open drive selection menu for the right panel (Alt+F2)
    DriveSelectRight,
}
