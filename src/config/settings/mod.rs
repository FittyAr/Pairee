use crate::app::state::{PanelViewMode, SortField};
use serde::{Deserialize, Serialize};

mod confirmations;
mod defaults;
#[cfg(test)]
mod tests;
pub mod types;

pub use confirmations::ConfirmationSettings;
pub use types::{
    PluginConfig, SshPreset, TabExpansion, default_auto_refresh_poll_secs,
    default_auto_refresh_ssh_poll_secs, default_compare_mtime_tolerance_secs,
    default_git_log_limit, default_plugins_dev_dir, default_ssh_timeout_secs,
    default_transfer_buffer, default_transfer_conflict, default_transfer_hash,
    default_transfer_max_retries, default_transfer_report_format, default_true,
    default_viewer_codepage,
};

/// User settings stored in `config.toml`.
///
/// Unknown keys are ignored when reading, so `config.toml` files written by
/// older releases (which stored options that were removed because they had no
/// effect, e.g. `default_editor` or `viewer_command`) still load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Whether to display hidden files/directories (starting with `.`)
    pub show_hidden: bool,
    /// Global Secure Mode boundary
    pub secure_mode: bool,
    /// Toggle terminal mouse interactions
    pub mouse_support: bool,
    /// False only on a freshly created config. Existing files without the
    /// field deserialize as `true` so upgrades skip the first-run dialog.
    #[serde(default = "default_true")]
    pub onboarding_completed: bool,
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

    // ── System settings (dialog tab 0) ───────────────────────────────────────
    pub delete_to_recycle_bin: bool,
    pub save_commands_history: bool,
    pub save_folders_history: bool,
    pub save_view_and_edit_history: bool,
    pub use_windows_registered_types: bool,
    pub automatic_update_env_variables: bool,
    pub req_admin_modification: bool,
    pub req_admin_reading: bool,
    pub sorting_collation: String,
    pub treat_digits_as_numbers: bool,
    pub case_sensitive_sort: bool,
    pub auto_save_setup: bool,

    // ── Panel settings (dialog tab 1) ────────────────────────────────────────
    pub highlight_files: bool,
    pub select_folders: bool,
    pub sort_folder_names_by_extension: bool,
    pub disable_panel_update_object_count: u32,
    /// Reread panels when their folder changes on disk (watch or poll).
    #[serde(default = "default_true")]
    pub auto_refresh: bool,
    /// Poll interval for folders that cannot be watched (network shares,
    /// folders above `disable_panel_update_object_count`).
    #[serde(default = "default_auto_refresh_poll_secs")]
    pub auto_refresh_poll_secs: u32,
    /// Also poll the folders of SFTP panels (one `stat` + `readdir` per
    /// interval on the shared connection).
    #[serde(default)]
    pub auto_refresh_ssh: bool,
    /// Seconds between checks of an SFTP folder.
    #[serde(default = "default_auto_refresh_ssh_poll_secs")]
    pub auto_refresh_ssh_poll_secs: u32,
    pub show_column_titles: bool,
    pub show_status_line: bool,
    pub show_files_total_information: bool,
    pub show_free_size: bool,
    pub show_scrollbar: bool,
    pub show_sort_mode_letter: bool,
    pub show_dotdot_in_root_folders: bool,
    /// Show the tab bar above a panel even when it has a single tab.
    #[serde(default)]
    pub always_show_tab_bar: bool,

    // ── Interface settings (dialog tab 2) ────────────────────────────────────
    pub interface_clock: bool,
    pub interface_show_key_bar: bool,
    pub interface_always_show_menu_bar: bool,
    pub auto_drop_menu: bool,
    pub enable_yazi_workflow: bool,
    /// Reopen the last session's tabs, layout and focused side at startup.
    #[serde(default = "default_true")]
    pub restore_session: bool,

    // ── Language & plugins settings (dialog tab 4) ───────────────────────────
    pub language: String,
    pub plugins_developer_mode: bool,
    #[serde(default = "default_plugins_dev_dir")]
    pub plugins_dev_dir: String,

    // ── Editor & viewer settings (dialog tab 5) ──────────────────────────────
    /// When `true`, pressing Enter on a file runs its file association
    /// command (e.g. `xdg-open %f`). When `false` (default), Enter opens the
    /// file in Pairee's native viewer for text, image, and binary files alike.
    /// Editing (F4) always uses the built-in editor.
    #[serde(default)]
    pub enter_use_external: bool,
    pub editor_expand_tabs: TabExpansion,
    pub editor_auto_indent: bool,
    pub editor_cursor_at_end: bool,
    pub editor_tab_size: u32,
    pub editor_show_line_numbers: bool,
    pub editor_lock_editing_readonly: bool,
    pub editor_warn_opening_readonly: bool,
    pub viewer_use_external: bool,
    pub viewer_tab_size: u32,
    pub viewer_show_scrollbar: bool,
    /// Detect the encoding of viewed files (byte-order mark, UTF-16, legacy
    /// code pages); when off, `viewer_default_codepage` is used.
    #[serde(default = "default_true")]
    pub viewer_autodetect_codepage: bool,
    /// Encoding name or label used when detection is off (e.g. `windows-1252`).
    #[serde(default = "default_viewer_codepage")]
    pub viewer_default_codepage: String,

    // ── Colors settings (dialog tab 6) ───────────────────────────────────────
    pub highlight_rules: Vec<crate::ui::highlight::HighlightRule>,

    #[serde(default)]
    pub ssh_presets: Vec<SshPreset>,

    // ── Git Integration settings ────────────────────────────────────────
    /// Whether the Git panel feature is enabled
    #[serde(default = "default_true")]
    pub git_enabled: bool,
    /// SSH/SFTP connect UI and actions
    #[serde(default = "default_true")]
    pub ssh_enabled: bool,
    /// Timeout in seconds for blocking SSH/SFTP calls (0 = no limit)
    #[serde(default = "default_ssh_timeout_secs")]
    pub ssh_timeout_secs: u64,
    /// Largest modification-time difference (seconds) that folder compare
    /// and synchronize still treat as equal (2 = FAT granularity)
    #[serde(default = "default_compare_mtime_tolerance_secs")]
    pub compare_mtime_tolerance_secs: u64,
    /// Load and run Lua plugins
    #[serde(default = "default_true")]
    pub plugins_enabled: bool,
    /// Decode images in the F3 viewer and quick view
    #[serde(default = "default_true")]
    pub image_preview_enabled: bool,
    /// Author name for commits (empty = read from git config)
    #[serde(default)]
    pub git_author_name: String,
    /// Author email for commits (empty = read from git config)
    #[serde(default)]
    pub git_author_email: String,
    /// Maximum number of commits to load in the log view
    #[serde(default = "default_git_log_limit")]
    pub git_log_limit: u32,

    // ── Auto-update settings ────────────────────────────────────────────────
    /// Whether Pairee should check GitHub Releases for updates on startup
    #[serde(default = "default_true")]
    pub auto_update_check: bool,
    /// If set, Pairee will not notify the user about this specific version tag
    #[serde(default)]
    pub dismissed_update_version: Option<String>,

    // ── Plugins settings ────────────────────────────────────────────────────
    #[serde(default)]
    pub plugins: std::collections::HashMap<String, PluginConfig>,
    #[serde(default)]
    pub plugin_settings:
        std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub active_dev_plugin: Option<String>,

    // ── Transfer Engine settings ─────────────────────────────────
    #[serde(default = "default_transfer_hash")]
    pub transfer_default_hash: String,
    #[serde(default = "default_transfer_buffer")]
    pub transfer_buffer_size: u32,
    #[serde(default)]
    pub transfer_verify_after_copy: bool,
    #[serde(default)]
    pub transfer_direct_io: bool,
    #[serde(default = "default_true")]
    pub transfer_preserve_timestamps: bool,
    #[serde(default = "default_true")]
    pub transfer_preserve_attributes: bool,
    #[serde(default = "default_transfer_max_retries")]
    pub transfer_max_retries: u32,
    #[serde(default = "default_transfer_conflict")]
    pub transfer_conflict_resolution: String,
    #[serde(default)]
    pub transfer_skip_symlinks: bool,
    #[serde(default)]
    pub transfer_halt_on_error: bool,
    #[serde(default)]
    pub transfer_preserve_acl: bool,
    #[serde(default)]
    pub transfer_preserve_streams: bool,
    #[serde(default)]
    pub transfer_follow_symlinks: bool,
    #[serde(default)]
    pub transfer_limit_bandwidth_rate: Option<u64>,
    #[serde(default)]
    pub transfer_auto_report: bool,
    #[serde(default = "default_transfer_report_format")]
    pub transfer_report_format: String,
}
