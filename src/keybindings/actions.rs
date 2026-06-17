use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // ── Navigation ──────────────────────────────────────────────────────────
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
    /// Select / tag the highlighted item for bulk operations (Insert / Space)
    SelectItem,
    /// Execute file action or enter subdirectory (Enter)
    Execute,
    /// Go to the parent directory (Backspace)
    GoParent,

    // ── Panel view modes (Ctrl+1 … Ctrl+0) ──────────────────────────────────
    /// Brief: filename-only, multi-column (Ctrl+1)
    PanelViewBrief,
    /// Medium: name + basic attributes (Ctrl+2)
    PanelViewMedium,
    /// Full: name + size + date (Ctrl+3)
    PanelViewFull,
    /// Wide: wide single column (Ctrl+4)
    PanelViewWide,
    /// Detailed: name + permissions + owner + real size (Ctrl+5)
    PanelViewDetailed,
    /// Descriptions: name + descript.ion entry (Ctrl+6)
    PanelViewDescriptions,
    /// File owners: name + owner/group (Ctrl+7)
    PanelViewFileOwners,
    /// File links: name + hardlink count (Ctrl+8)
    PanelViewFileLinks,
    /// Alternative full: user-configurable columns (Ctrl+9)
    PanelViewAltFull,

    // ── Panel toggles ────────────────────────────────────────────────────────
    /// Toggle left panel visibility (Ctrl+F1)
    TogglePanelLeft,
    /// Toggle right panel visibility (Ctrl+F2)
    TogglePanelRight,
    /// Toggle both panels (Ctrl+O)
    ToggleBothPanels,
    /// Activate Info panel mode (Ctrl+L)
    InfoPanel,
    /// Activate Quick View mode in passive panel (Ctrl+Q)
    QuickView,
    /// Open Sort Modes dialog (Ctrl+F12)
    SortModes,

    // ── Sorting ──────────────────────────────────────────────────────────────
    /// Sort by Name (Ctrl+F3)
    SortByName,
    /// Sort by Extension (Ctrl+F4)
    SortByExtension,
    /// Sort by Write Time (Ctrl+F5)
    SortByWriteTime,
    /// Sort by Size (Ctrl+F6)
    SortBySize,
    /// Unsorted (Ctrl+F7)
    SortUnsorted,
    /// Sort by Creation Time (Ctrl+F8)
    SortByCreationTime,
    /// Sort by Access Time (Ctrl+F9)
    SortByAccessTime,
    /// Sort by Description (Ctrl+F10)
    SortByDescription,
    /// Sort by Owner (Ctrl+F11)
    SortByOwner,

    // ── F-key actions ────────────────────────────────────────────────────────
    /// Open help dialog (F1)
    Help,
    /// Open user custom commands menu (F2)
    UserMenu,
    /// View file content (F3)
    View,
    /// View file content alternative (Alt+F3)
    ViewAlt,
    /// Edit file (F4)
    Edit,
    /// Copy selected items to the other panel (F5)
    Copy,
    /// Rename or Move selected items (F6)
    Move,
    /// Make directory (F7)
    MkDir,
    /// Delete selected items (F8)
    Delete,
    /// Open top pulldown menu (F9)
    Menu,
    /// Quit application (F10)
    Quit,
    /// Open Plugin commands menu (F11)
    PluginMenu,
    /// Show screens list (F12)
    ScreensList,
    /// Next Screen (Ctrl+Tab)
    NextScreen,
    /// Previous Screen (Ctrl+Shift+Tab)
    PrevScreen,

    // ── File operations ──────────────────────────────────────────────────────
    /// Print file (Alt+F5)
    PrintFile,
    /// Create a symbolic or hard link to the selected item (Alt+F6)
    CreateLink,
    /// Secure overwrite-then-delete (Alt+Del)
    WipeFile,
    /// View/edit file attributes: permissions, dates (Ctrl+A)
    FileAttributes,
    /// Apply a shell command template to all selected files (Ctrl+G)
    ApplyCommand,
    /// Add/edit a short description for the selected file (Ctrl+Z)
    DescribeFile,
    /// Add to archive (Shift+F1)
    CompressFiles,
    /// Extract archive (Shift+F2)
    ExtractArchive,
    /// Archive commands: verify/add/delete inside archive (Shift+F3)
    ArchiveCommands,

    // ── Bulk selection ────────────────────────────────────────────────────────
    /// Select files matching a glob mask (Gray +)
    SelectGroup,
    /// Unselect files matching a glob mask (Gray -)
    UnselectGroup,
    /// Invert selection of all files (Gray *)
    InvertSelection,
    /// Restore last bulk selection snapshot (Ctrl+M)
    RestoreSelection,

    // ── Search & history ─────────────────────────────────────────────────────
    /// Global file search by name and/or content (Alt+F7)
    FindFile,
    /// Show command-line history (Alt+F8)
    CommandHistory,
    /// Show file view history (Alt+F11)
    FileViewHistory,
    /// Show folders history (Alt+F12)
    FoldersHistory,

    // ── Commands ─────────────────────────────────────────────────────────────
    /// Compare contents of left and right panels (Commands menu)
    CompareFolder,
    /// Open user menu editor (Commands menu)
    EditUserMenu,
    /// Open file associations editor (Commands menu)
    FileAssociations,
    /// Configure folder shortcuts (Commands menu)
    FolderShortcutsConfig,
    /// Set a permanent file mask filter on the active panel (Ctrl+I)
    FilePanelFilter,
    /// Show list of running OS processes (Ctrl+W)
    TaskList,

    // ── Options ──────────────────────────────────────────────────────────────
    /// Save all settings and state immediately (Shift+F9)
    SaveSetup,
    /// Open system settings dialog
    SystemSettings,

    // ── General ─────────────────────────────────────────────────────────────
    /// Toggle visibility of dotfiles/hidden files (Ctrl+H)
    ToggleHidden,
    /// Focus cursor on terminal Command Line Input
    FocusCli,
    /// Unfocus CLI / clear search filters / close popup (Esc)
    Unfocus,
    /// Refresh the files in both panels (Ctrl+R)
    Refresh,
    /// Swap positions of left and right panels (Ctrl+U)
    SwapPanels,
    /// Open drive selection menu for the left panel (Alt+F1)
    DriveSelectLeft,
    /// Open drive selection menu for the right panel (Alt+F2)
    DriveSelectRight,
    /// Open Context Menu for targeted item(s)
    ContextMenu,
    /// Navigate to a numbered folder shortcut (1–9)
    GoFolderShortcut(u8),
    /// Toggle show/hide long file names (Ctrl+N)
    ToggleLongNames,
    /// Re-read (refresh) the active panel only (Ctrl+R when not in conflict)
    RereadPanel,
    /// Video mode (resolution config) — Alt+F9
    VideoMode,
    /// Open graphical directory tree navigator (Alt+F10)
    TreeView,
    /// Cycle active F-key modifier row view (Normal -> Ctrl -> Alt)
    CycleFKeysModifiers,
    /// Connect active panel to a remote server via SSH
    SshConnect,
    /// Disconnect active panel from SSH
    SshDisconnect,
}
