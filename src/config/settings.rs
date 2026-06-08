use serde::{Deserialize, Serialize};
use crate::app::state::{PanelViewMode, SortField};

/// Confirmation settings — which operations require an explicit confirmation dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationSettings {
    pub confirm_delete: bool,
    pub confirm_overwrite: bool,
    pub confirm_wipe: bool,
    pub confirm_quit: bool,
}

impl Default for ConfirmationSettings {
    fn default() -> Self {
        Self {
            confirm_delete: true,
            confirm_overwrite: true,
            confirm_wipe: true,
            confirm_quit: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Whether to display hidden files/directories (starting with `.`)
    pub show_hidden: bool,
    /// The external editor command to trigger for F4 Edit (e.g. "nano", "vim")
    pub default_editor: String,
    /// Toggle terminal mouse interactions
    pub mouse_support: bool,
    /// Active keybinding preset profile: "norton", "vim", "modern", "custom"
    pub keybinding_preset: String,
    /// The name of the active theme
    pub theme: String,

    // ── Panel view defaults ──────────────────────────────────────────────────
    /// Default view mode applied when the app starts
    pub panel_view_mode: PanelViewMode,
    /// Default sort field
    pub sort_field: SortField,
    /// Sort in reverse order by default
    pub sort_reverse: bool,
    /// Show full long file names by default (true) or truncate (false)
    pub show_long_names: bool,

    // ── Panel visibility defaults ────────────────────────────────────────────
    pub left_panel_visible: bool,
    pub right_panel_visible: bool,

    // ── Confirmations ────────────────────────────────────────────────────────
    pub confirmations: ConfirmationSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_editor: if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                "nano".to_string()
            },
            mouse_support: true,
            keybinding_preset: "norton".to_string(),
            theme: "slate".to_string(),
            panel_view_mode: PanelViewMode::default(),
            sort_field: SortField::default(),
            sort_reverse: false,
            show_long_names: true,
            left_panel_visible: true,
            right_panel_visible: true,
            confirmations: ConfirmationSettings::default(),
        }
    }
}
