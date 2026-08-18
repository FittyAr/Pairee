use crate::app::state::{PanelViewMode, SortField};
use serde::{Deserialize, Serialize};

mod confirmations;
mod defaults;

pub use confirmations::ConfirmationSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Whether to display hidden files/directories (starting with `.`)
    pub show_hidden: bool,
    /// Global Secure Mode boundary
    pub secure_mode: bool,
    /// The external editor command to trigger for F4 Edit (e.g. "nano", "vim")
    pub default_editor: String,
    /// Toggle terminal mouse interactions
    pub mouse_support: bool,
    /// Active keybinding preset profile: "norton", "vim", "modern", "custom"
    pub keybinding_preset: String,
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

    // ── NEW System settings (Tab 0 stubs/interactive) ────────────────────────
    pub delete_to_recycle_bin: bool,
    pub use_system_copy_routine: bool,
    pub copy_files_opened_for_writing: bool,
    pub scan_symbolic_links: bool,
    pub save_commands_history: bool,
    pub save_folders_history: bool,
    pub save_view_and_edit_history: bool,
    pub use_windows_registered_types: bool,
    pub automatic_update_env_variables: bool,
    pub req_admin_modification: bool,
    pub req_admin_reading: bool,
    pub req_admin_use_additional_privileges: bool,
    pub sorting_collation: String,
    pub treat_digits_as_numbers: bool,
    pub case_sensitive_sort: bool,
    pub auto_save_setup: bool,

    // ── NEW Panel settings (Tab 1 stubs/interactive) ─────────────────────────
    pub highlight_files: bool,
    pub select_folders: bool,
    pub right_click_selects_files: bool,
    pub sort_folder_names_by_extension: bool,
    pub disable_panel_update_object_count: u32,
    pub network_drives_autorefresh: bool,
    pub show_column_titles: bool,
    pub show_status_line: bool,
    pub detect_volume_mount_points: bool,
    pub show_files_total_information: bool,
    pub show_free_size: bool,
    pub show_scrollbar: bool,
    pub show_background_screens_number: bool,
    pub show_sort_mode_letter: bool,
    pub show_dotdot_in_root_folders: bool,
    pub infopanel_show_power_status: bool,
    pub infopanel_show_cd_drive_parameters: bool,
    pub infopanel_computer_name_format: String,
    pub infopanel_user_name_format: String,
    pub file_descriptions_list_names: String,
    pub file_descriptions_set_hidden: bool,
    pub file_descriptions_update_readonly: bool,
    pub file_descriptions_position: u32,
    pub file_descriptions_update_mode: String,
    pub file_descriptions_use_ansi: bool,
    pub file_descriptions_save_utf8: bool,
    pub folder_description_list_names: String,

    // ── NEW Interface settings (Tab 2 stubs/interactive) ─────────────────────
    pub interface_clock: bool,
    pub interface_show_key_bar: bool,
    pub interface_always_show_menu_bar: bool,
    pub interface_screen_saver_minutes: u32,
    pub interface_show_total_copy_progress: bool,
    pub interface_show_copying_time: bool,
    pub interface_show_total_delete_progress: bool,
    pub interface_use_ctrl_pgup_change_drive: bool,
    pub auto_drop_menu: bool,
    pub interface_use_virtual_terminal: bool,
    pub interface_fullwidth_aware_rendering: bool,
    pub interface_cleartype_friendly_redraw: bool,
    pub interface_console_icon: u32,
    pub interface_console_icon_admin_alternate: bool,
    pub interface_window_title_addons: String,
    pub dialog_history_in_edit_controls: bool,
    pub dialog_persistent_blocks: bool,
    pub dialog_del_removes_blocks: bool,
    pub dialog_autocomplete: bool,
    pub dialog_backspace_deletes_unchanged: bool,
    pub dialog_mouse_click_outside_closes: bool,
    pub menu_left_click_outside: String,
    pub menu_right_click_outside: String,
    pub menu_middle_click_outside: String,
    pub cmdline_persistent_blocks: bool,
    pub cmdline_del_removes_blocks: bool,
    pub cmdline_autocomplete: bool,
    pub cmdline_prompt_format: String,
    pub cmdline_use_home_dir: String,
    pub autocomplete_show_list: bool,
    pub autocomplete_modal_mode: bool,
    pub autocomplete_append_first: bool,
    pub enable_yazi_workflow: bool,

    // ── NEW Language & Plugins settings (Tab 4 stubs/interactive) ────────────
    pub language: String,
    pub plugins_manager_oem_support: bool,
    pub plugins_manager_scan_symlinks: bool,
    pub plugins_manager_file_processing: bool,
    pub plugins_manager_show_standard_association: bool,
    pub plugins_manager_even_if_one_found: bool,
    pub plugins_manager_search_results: bool,
    pub plugins_manager_prefix_processing: bool,
    pub plugins_developer_mode: bool,
    #[serde(default = "default_plugins_dev_dir")]
    pub plugins_dev_dir: String,

    // ── NEW Editor & Viewer settings (Tab 5 stubs/interactive) ───────────────
    /// When `true`, pressing Enter on a file runs the external association
    /// command (e.g. `nano %f`). When `false` (default), Enter opens the file
    /// in Pairee's native viewer for text, image, and binary files alike.
    #[serde(default)]
    pub enter_use_external: bool,
    pub editor_use_external: bool,
    pub editor_expand_tabs: String,
    pub editor_persistent_blocks: bool,
    pub editor_cursor_beyond_eol: bool,
    pub editor_del_removes_blocks: bool,
    pub editor_select_found: bool,
    pub editor_auto_indent: bool,
    pub editor_cursor_at_end: bool,
    pub editor_tab_size: u32,
    pub editor_show_scrollbar: bool,
    pub editor_show_white_space: bool,
    pub editor_show_line_numbers: bool,
    pub editor_save_file_position: bool,
    pub editor_save_bookmarks: bool,
    pub editor_allow_editing_opened_writing: bool,
    pub editor_lock_editing_readonly: bool,
    pub editor_warn_opening_readonly: bool,
    pub editor_autodetect_codepage: bool,
    pub editor_default_codepage: String,
    pub viewer_use_external: bool,
    pub viewer_command: String,
    pub viewer_persistent_selection: bool,
    pub viewer_show_scrolling_arrows: bool,
    pub viewer_tab_size: u32,
    pub viewer_visible_zero: bool,
    pub viewer_show_scrollbar: bool,
    pub viewer_save_file_position: bool,
    pub viewer_save_view_mode: bool,
    pub viewer_save_file_codepage: bool,
    pub viewer_save_wrap_mode: bool,
    pub viewer_save_bookmarks: bool,
    pub viewer_detect_dump_view_mode: bool,
    pub viewer_max_line_width: u32,
    pub viewer_autodetect_codepage: bool,
    pub viewer_default_codepage: String,

    // ── NEW Colors settings (Tab 6 interactive) ──────────────────────────────
    pub highlight_rules: Vec<crate::ui::highlight::HighlightRule>,

    #[serde(default)]
    pub ssh_presets: Vec<SshPreset>,

    // ── Git Integration settings ────────────────────────────────────────
    /// Whether the Git panel feature is enabled
    #[serde(default = "default_true")]
    pub git_enabled: bool,
    /// Auto-detect git repos when changing directory
    #[serde(default = "default_true")]
    pub git_auto_detect: bool,
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
    #[serde(default = "default_true")]
    pub transfer_engine_enabled: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshPreset {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

// ── Serde default helpers ────────────────────────────────────────────────────

fn default_true() -> bool {
    true
}

fn default_git_log_limit() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PluginConfig {
    pub name: String,
    #[serde(default)]
    pub trusted: bool,
}

pub(super) fn default_plugins_dev_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            std::path::PathBuf::from(appdata)
                .join("pairee")
                .join("config")
                .join("plugins")
                .to_string_lossy()
                .into_owned()
        } else {
            "./config/plugins".to_string()
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        crate::config::paths::get_config_dir()
            .join("plugins")
            .to_string_lossy()
            .into_owned()
    }
}

fn default_transfer_hash() -> String {
    "blake3".to_string()
}
fn default_transfer_buffer() -> u32 {
    1024 * 1024
}
fn default_transfer_max_retries() -> u32 {
    3
}
fn default_transfer_conflict() -> String {
    "ask".to_string()
}
fn default_transfer_report_format() -> String {
    "html".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_install_default_needs_onboarding() {
        assert!(!Settings::default().onboarding_completed);
    }

    #[test]
    fn existing_config_without_field_skips_onboarding() {
        let mut table: toml::Table =
            toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
        table.remove("onboarding_completed");
        let stripped = toml::to_string(&table).unwrap();
        let loaded: Settings = toml::from_str(&stripped).unwrap();
        assert!(
            loaded.onboarding_completed,
            "missing field must mean already onboarded"
        );
    }
}
