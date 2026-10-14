//! Menus and pickers: the top menu, user menu, context and archive menus,
//! drive, hotlist and folder shortcut pickers, sort modes, the yazi key
//! boards, the screens list, the command palette and the first-run keymap
//! picker. A click outside closes most of them.

use crate::app::state::types::{ActivePanel, SortField};
use crate::app::text_input::TextField;
use std::path::PathBuf;

/// User menu (F2). Its items are read from `usermenu.toml` once, when it
/// opens (`app::input_popup::user_menu::open`), not on every key or frame.
#[derive(Debug, Clone)]
pub struct UserMenu {
    pub cursor_idx: usize,
    pub items: Vec<UserMenuItem>,
}

/// One user menu entry: a shell command or an action, on a shortcut key.
#[derive(Debug, Clone)]
pub struct UserMenuItem {
    pub key: String,
    pub label: String,
    pub command: Option<String>,
    pub action: Option<crate::keybindings::Action>,
}

/// Top menu bar with its open drop-down (and submenu).
#[derive(Debug, Clone)]
pub struct Menu {
    pub active_menu_idx: usize,
    pub active_item_idx: Option<usize>,
    pub active_submenu_idx: Option<usize>,
    pub active_submenu_item_idx: Option<usize>,
}

/// Sort keys board of the yazi workflow.
#[derive(Debug, Clone)]
pub struct YaziSortPopup;

/// View keys board of the yazi workflow.
#[derive(Debug, Clone)]
pub struct YaziViewPopup;

/// Context menu of the panel entry.
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub items: Vec<String>,
    pub cursor_idx: usize,
}

/// Drive picker of one panel (Alt+F1 / Alt+F2).
#[derive(Debug, Clone)]
pub struct DriveSelect {
    pub panel: ActivePanel,
    pub drives: Vec<String>,
    pub cursor_idx: usize,
}

/// Folder hotlist (bookmarks).
#[derive(Debug, Clone)]
pub struct Hotlist {
    pub entries: Vec<crate::config::bookmarks::HotlistEntry>,
    pub cursor_idx: usize,
}

/// Folder shortcut slots 1–9 (contents live in `AppState::folder_shortcuts`).
#[derive(Debug, Clone)]
pub struct FolderShortcuts {
    pub cursor_idx: usize,
}

/// Sort modes (Ctrl+F12).
#[derive(Debug, Clone)]
pub struct SortModesDialog {
    pub current: SortField,
    pub reverse: bool,
    pub cursor_idx: usize,
}

/// Screens list (F12), pushed over the dialogs of the current screen.
#[derive(Debug, Clone)]
pub struct ScreensMenu {
    pub cursor_idx: usize,
}

/// Commands of an archive (Shift+F9).
#[derive(Debug, Clone)]
pub struct ArchiveCommandsMenu {
    pub archive_path: PathBuf,
    pub items: Vec<String>,
    pub cursor_idx: usize,
}

/// Command palette: actions filtered by the query.
#[derive(Debug, Clone)]
pub struct CommandPalette {
    pub query: TextField,
    pub cursor_idx: usize,
    pub items: Vec<(String, crate::keybindings::Action)>,
}

/// First-run keymap preset picker.
#[derive(Debug, Clone)]
pub struct OnboardingKeymap {
    pub cursor_idx: usize,
}
