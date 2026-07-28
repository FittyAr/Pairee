use crate::app::state::{PanelViewMode, SortField};
use serde::{Deserialize, Serialize};

/// Confirmation settings — which operations require an explicit confirmation dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationSettings {
    pub confirm_delete: bool,
    pub confirm_overwrite: bool,
    pub confirm_wipe: bool,
    pub confirm_quit: bool,
    // Stubs from screenshots
    pub confirm_copy: bool,
    pub confirm_move: bool,
    pub confirm_drag_and_drop: bool,
    pub confirm_delete_non_empty_folders: bool,
    pub confirm_interrupt_operation: bool,
    pub confirm_disconnect_network_drive: bool,
    pub confirm_delete_subst_disk: bool,
    pub confirm_detach_virtual_disk: bool,
    pub confirm_hotplug_removal: bool,
    pub confirm_reload_edited_file: bool,
    pub confirm_clear_history_list: bool,
}

impl Default for ConfirmationSettings {
    fn default() -> Self {
        Self {
            confirm_delete: true,
            confirm_overwrite: true,
            confirm_wipe: true,
            confirm_quit: false,
            confirm_copy: true,
            confirm_move: true,
            confirm_drag_and_drop: true,
            confirm_delete_non_empty_folders: true,
            confirm_interrupt_operation: true,
            confirm_disconnect_network_drive: true,
            confirm_delete_subst_disk: true,
            confirm_detach_virtual_disk: true,
            confirm_hotplug_removal: true,
            confirm_reload_edited_file: true,
            confirm_clear_history_list: true,
        }
    }
}

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

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_hidden: false,
            secure_mode: false,
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

            // Tab 0
            delete_to_recycle_bin: false,
            use_system_copy_routine: false,
            copy_files_opened_for_writing: false,
            scan_symbolic_links: false,
            save_commands_history: true,
            save_folders_history: true,
            save_view_and_edit_history: true,
            use_windows_registered_types: false,
            automatic_update_env_variables: false,
            req_admin_modification: false,
            req_admin_reading: false,
            req_admin_use_additional_privileges: false,
            sorting_collation: "linguistic".to_string(),
            treat_digits_as_numbers: false,
            case_sensitive_sort: false,
            auto_save_setup: false,

            // Tab 1
            highlight_files: true,
            select_folders: true,
            right_click_selects_files: false,
            sort_folder_names_by_extension: false,
            disable_panel_update_object_count: 0,
            network_drives_autorefresh: true,
            show_column_titles: true,
            show_status_line: true,
            detect_volume_mount_points: false,
            show_files_total_information: true,
            show_free_size: false,
            show_scrollbar: false,
            show_background_screens_number: true,
            show_sort_mode_letter: true,
            show_dotdot_in_root_folders: false,
            infopanel_show_power_status: false,
            infopanel_show_cd_drive_parameters: true,
            infopanel_computer_name_format: "Physical NetBIOS".to_string(),
            infopanel_user_name_format: "Logon name".to_string(),
            file_descriptions_list_names: "Descript.ion,Files.bbs".to_string(),
            file_descriptions_set_hidden: true,
            file_descriptions_update_readonly: false,
            file_descriptions_position: 0,
            file_descriptions_update_mode: "Update if displayed".to_string(),
            file_descriptions_use_ansi: false,
            file_descriptions_save_utf8: false,
            folder_description_list_names: "DirInfo,File_Id.diz,Descript.ion,ReadMe.*,Read.Me"
                .to_string(),

            // Tab 2
            interface_clock: true,
            interface_show_key_bar: true,
            interface_always_show_menu_bar: false,
            interface_screen_saver_minutes: 5,
            interface_show_total_copy_progress: true,
            interface_show_copying_time: true,
            interface_show_total_delete_progress: false,
            interface_use_ctrl_pgup_change_drive: true,
            auto_drop_menu: false,
            interface_use_virtual_terminal: false,
            interface_fullwidth_aware_rendering: false,
            interface_cleartype_friendly_redraw: true,
            interface_console_icon: 0,
            interface_console_icon_admin_alternate: true,
            interface_window_title_addons: "%Ver %Platform %Admin".to_string(),
            dialog_history_in_edit_controls: true,
            dialog_persistent_blocks: false,
            dialog_del_removes_blocks: true,
            dialog_autocomplete: true,
            dialog_backspace_deletes_unchanged: false,
            dialog_mouse_click_outside_closes: true,
            menu_left_click_outside: "Cancel menu".to_string(),
            menu_right_click_outside: "Cancel menu".to_string(),
            menu_middle_click_outside: "Execute selected item".to_string(),
            cmdline_persistent_blocks: false,
            cmdline_del_removes_blocks: true,
            cmdline_autocomplete: true,
            cmdline_prompt_format: "$p$g".to_string(),
            cmdline_use_home_dir: "%FARHOME%".to_string(),
            autocomplete_show_list: true,
            autocomplete_modal_mode: false,
            autocomplete_append_first: false,
            enable_yazi_workflow: false,

            // Tab 4
            language: "English".to_string(),
            plugins_manager_oem_support: true,
            plugins_manager_scan_symlinks: true,
            plugins_manager_file_processing: false,
            plugins_manager_show_standard_association: false,
            plugins_manager_even_if_one_found: false,
            plugins_manager_search_results: false,
            plugins_manager_prefix_processing: false,
            plugins_developer_mode: false,
            plugins_dev_dir: default_plugins_dev_dir(),

            // Tab 5
            enter_use_external: false,
            editor_use_external: false,
            editor_expand_tabs: "Do not expand tabs".to_string(),
            editor_persistent_blocks: false,
            editor_cursor_beyond_eol: true,
            editor_del_removes_blocks: true,
            editor_select_found: false,
            editor_auto_indent: false,
            editor_cursor_at_end: false,
            editor_tab_size: 8,
            editor_show_scrollbar: false,
            editor_show_white_space: false,
            editor_show_line_numbers: false,
            editor_save_file_position: true,
            editor_save_bookmarks: true,
            editor_allow_editing_opened_writing: true,
            editor_lock_editing_readonly: false,
            editor_warn_opening_readonly: false,
            editor_autodetect_codepage: true,
            editor_default_codepage: "1252".to_string(),
            viewer_use_external: false,
            viewer_command: "".to_string(),
            viewer_persistent_selection: true,
            viewer_show_scrolling_arrows: true,
            viewer_tab_size: 8,
            viewer_visible_zero: false,
            viewer_show_scrollbar: false,
            viewer_save_file_position: true,
            viewer_save_view_mode: true,
            viewer_save_file_codepage: true,
            viewer_save_wrap_mode: false,
            viewer_save_bookmarks: true,
            viewer_detect_dump_view_mode: true,
            viewer_max_line_width: 10000,
            viewer_autodetect_codepage: true,
            viewer_default_codepage: "1252".to_string(),

            // Tab 6
            highlight_rules: crate::ui::highlight::default_highlight_rules(),
            ssh_presets: Vec::new(),

            // Git integration
            git_enabled: true,
            git_auto_detect: true,
            git_author_name: String::new(),
            git_author_email: String::new(),
            git_log_limit: 100,

            // Update
            auto_update_check: true,
            dismissed_update_version: None,

            // Plugins
            plugins: std::collections::HashMap::new(),
            plugin_settings: std::collections::HashMap::new(),
            active_dev_plugin: None,

            // Transfer Engine
            transfer_engine_enabled: true,
            transfer_default_hash: "blake3".to_string(),
            transfer_buffer_size: 1024 * 1024,
            transfer_verify_after_copy: false,
            transfer_direct_io: false,
            transfer_preserve_timestamps: true,
            transfer_preserve_attributes: true,
            transfer_max_retries: 3,
            transfer_conflict_resolution: "ask".to_string(),
            transfer_skip_symlinks: false,
            transfer_halt_on_error: false,
            transfer_preserve_acl: false,
            transfer_preserve_streams: false,
            transfer_follow_symlinks: false,
            transfer_limit_bandwidth_rate: None,
            transfer_auto_report: false,
            transfer_report_format: "html".to_string(),
        }
    }
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

fn default_plugins_dev_dir() -> String {
    // The previous Windows branch produced `<APPDATA>/pairee/config/plugins`
    // (3 levels deep) while the Unix branch produced `<config_dir>/plugins`
    // (1 level). That asymmetry violated the AGENTS.md rule "do not
    // add extra levels of nested project directories" and forced the
    // Windows installer to be aware of the Linux layout. We now use
    // the same `config_dir + "plugins"` layout on every platform.
    crate::config::paths::get_config_dir()
        .join("plugins")
        .to_string_lossy()
        .into_owned()
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

    // ── Default helpers (these are the only stable public knobs) ───────────

    #[test]
    fn default_helpers_have_documented_values() {
        assert!(default_true());
        assert_eq!(default_git_log_limit(), 100);
        assert_eq!(default_plugins_dev_dir().ends_with("plugins"), true);
        assert_eq!(default_transfer_hash(), "blake3");
        assert_eq!(default_transfer_buffer(), 1024 * 1024);
        assert_eq!(default_transfer_max_retries(), 3);
        assert_eq!(default_transfer_conflict(), "ask");
        assert_eq!(default_transfer_report_format(), "html");
    }

    // ── ConfirmationSettings::default ───────────────────────────────────────

    #[test]
    fn confirmation_settings_default_is_safe() {
        let c = ConfirmationSettings::default();
        // Destructive actions confirm by default.
        assert!(c.confirm_delete);
        assert!(c.confirm_wipe);
        assert!(c.confirm_delete_non_empty_folders);
        // Quit does not confirm — pressing F10 is too common.
        assert!(!c.confirm_quit);
    }

    // ── Settings::default ──────────────────────────────────────────────────

    #[test]
    fn settings_default_has_no_panic_in_construction() {
        // Just constructing the default exercises every `default_*`
        // helper. If any of them is accidentally non-`const`, this
        // stops compiling.
        let s = Settings::default();
        // Spot-check a handful of values that other code relies on.
        assert!(!s.secure_mode);
        assert_eq!(s.keybinding_preset, "norton");
        assert_eq!(s.theme, "slate");
        assert!(s.left_panel_visible);
        assert!(s.right_panel_visible);
        assert!(!s.auto_drop_menu);
        // The git log limit is fixed at 100 — many UI panels assume it.
        assert_eq!(s.git_log_limit, 100);
        // Auto-update is on by default.
        assert!(s.auto_update_check);
        // SSH presets: none until the user adds some.
        assert!(s.ssh_presets.is_empty());
        // Highlight rules: pulled from the default rules helper.
        assert!(!s.highlight_rules.is_empty());
    }

    // ── Serde roundtrip ────────────────────────────────────────────────────

    #[test]
    fn settings_serde_roundtrip_preserves_all_fields() {
        let original = Settings {
            show_hidden: true,
            secure_mode: true,
            default_editor: "code".to_string(),
            mouse_support: false,
            keybinding_preset: "vim".to_string(),
            theme: "solarized".to_string(),
            panel_view_mode: PanelViewMode::Brief,
            sort_field: SortField::Size,
            sort_reverse: true,
            show_long_names: false,
            left_panel_visible: false,
            right_panel_visible: true,
            confirmations: ConfirmationSettings {
                confirm_delete: false,
                confirm_overwrite: false,
                confirm_wipe: false,
                confirm_quit: true,
                confirm_copy: false,
                confirm_move: false,
                confirm_drag_and_drop: false,
                confirm_delete_non_empty_folders: false,
                confirm_interrupt_operation: false,
                confirm_disconnect_network_drive: false,
                confirm_delete_subst_disk: false,
                confirm_detach_virtual_disk: false,
                confirm_hotplug_removal: false,
                confirm_reload_edited_file: false,
                confirm_clear_history_list: false,
            },
            delete_to_recycle_bin: true,
            use_system_copy_routine: true,
            copy_files_opened_for_writing: true,
            scan_symbolic_links: true,
            save_commands_history: false,
            save_folders_history: false,
            save_view_and_edit_history: false,
            use_windows_registered_types: true,
            automatic_update_env_variables: true,
            req_admin_modification: true,
            req_admin_reading: true,
            req_admin_use_additional_privileges: true,
            sorting_collation: "ascii".to_string(),
            treat_digits_as_numbers: true,
            case_sensitive_sort: true,
            auto_save_setup: true,
            highlight_files: false,
            select_folders: false,
            right_click_selects_files: true,
            sort_folder_names_by_extension: true,
            disable_panel_update_object_count: 999,
            network_drives_autorefresh: false,
            show_column_titles: false,
            show_status_line: false,
            detect_volume_mount_points: true,
            show_files_total_information: false,
            show_free_size: true,
            show_scrollbar: true,
            show_background_screens_number: false,
            show_sort_mode_letter: false,
            show_dotdot_in_root_folders: true,
            infopanel_show_power_status: true,
            infopanel_show_cd_drive_parameters: false,
            infopanel_computer_name_format: "DNS".to_string(),
            infopanel_user_name_format: "Full name".to_string(),
            file_descriptions_list_names: "X,Y,Z".to_string(),
            file_descriptions_set_hidden: false,
            file_descriptions_update_readonly: true,
            file_descriptions_position: 5,
            file_descriptions_update_mode: "Always".to_string(),
            file_descriptions_use_ansi: true,
            file_descriptions_save_utf8: true,
            folder_description_list_names: "Readme".to_string(),
            interface_clock: false,
            interface_show_key_bar: false,
            interface_always_show_menu_bar: true,
            interface_screen_saver_minutes: 30,
            interface_show_total_copy_progress: false,
            interface_show_copying_time: false,
            interface_show_total_delete_progress: true,
            interface_use_ctrl_pgup_change_drive: false,
            auto_drop_menu: true,
            interface_use_virtual_terminal: true,
            interface_fullwidth_aware_rendering: true,
            interface_cleartype_friendly_redraw: false,
            interface_console_icon: 42,
            interface_console_icon_admin_alternate: false,
            interface_window_title_addons: " - test".to_string(),
            dialog_history_in_edit_controls: false,
            dialog_persistent_blocks: true,
            dialog_del_removes_blocks: false,
            dialog_autocomplete: false,
            dialog_backspace_deletes_unchanged: true,
            dialog_mouse_click_outside_closes: false,
            menu_left_click_outside: "Confirm".to_string(),
            menu_right_click_outside: "Confirm".to_string(),
            menu_middle_click_outside: "Ignore".to_string(),
            cmdline_persistent_blocks: true,
            cmdline_del_removes_blocks: false,
            cmdline_autocomplete: false,
            cmdline_prompt_format: "$G".to_string(),
            cmdline_use_home_dir: "$HOME".to_string(),
            autocomplete_show_list: false,
            autocomplete_modal_mode: true,
            autocomplete_append_first: true,
            enable_yazi_workflow: true,
            language: "Español".to_string(),
            plugins_manager_oem_support: false,
            plugins_manager_scan_symlinks: false,
            plugins_manager_file_processing: true,
            plugins_manager_show_standard_association: true,
            plugins_manager_even_if_one_found: true,
            plugins_manager_search_results: true,
            plugins_manager_prefix_processing: true,
            plugins_developer_mode: true,
            plugins_dev_dir: "/tmp/dev-plugins".to_string(),
            enter_use_external: true,
            editor_use_external: true,
            editor_expand_tabs: "Always".to_string(),
            editor_persistent_blocks: true,
            editor_cursor_beyond_eol: false,
            editor_del_removes_blocks: false,
            editor_select_found: true,
            editor_auto_indent: true,
            editor_cursor_at_end: true,
            editor_tab_size: 4,
            editor_show_scrollbar: true,
            editor_show_white_space: true,
            editor_show_line_numbers: true,
            editor_save_file_position: false,
            editor_save_bookmarks: false,
            editor_allow_editing_opened_writing: false,
            editor_lock_editing_readonly: true,
            editor_warn_opening_readonly: true,
            editor_autodetect_codepage: false,
            editor_default_codepage: "65001".to_string(),
            viewer_use_external: true,
            viewer_command: "less".to_string(),
            viewer_persistent_selection: false,
            viewer_show_scrolling_arrows: false,
            viewer_tab_size: 2,
            viewer_visible_zero: true,
            viewer_show_scrollbar: true,
            viewer_save_file_position: false,
            viewer_save_view_mode: false,
            viewer_save_file_codepage: false,
            viewer_save_wrap_mode: true,
            viewer_save_bookmarks: false,
            viewer_detect_dump_view_mode: false,
            viewer_max_line_width: 200,
            viewer_autodetect_codepage: false,
            viewer_default_codepage: "65001".to_string(),
            highlight_rules: vec![],
            ssh_presets: vec![SshPreset {
                name: "prod".to_string(),
                host: "example.com".to_string(),
                port: "22".to_string(),
                username: "ops".to_string(),
                password: None,
                key_path: Some("/home/ops/.ssh/id_ed25519".to_string()),
            }],
            git_enabled: false,
            git_auto_detect: false,
            git_author_name: "Alice".to_string(),
            git_author_email: "alice@example.com".to_string(),
            git_log_limit: 250,
            auto_update_check: false,
            dismissed_update_version: Some("v0.7.0".to_string()),
            plugins: std::collections::HashMap::from([(
                "my-plugin".to_string(),
                PluginConfig {
                    name: "my-plugin".to_string(),
                    trusted: true,
                },
            )]),
            plugin_settings: std::collections::HashMap::from([(
                "my-plugin".to_string(),
                std::collections::HashMap::from([("key".to_string(), "v".to_string())]),
            )]),
            active_dev_plugin: Some("my-plugin".to_string()),
            transfer_engine_enabled: false,
            transfer_default_hash: "sha256".to_string(),
            transfer_buffer_size: 256 * 1024,
            transfer_verify_after_copy: true,
            transfer_direct_io: true,
            transfer_preserve_timestamps: false,
            transfer_preserve_attributes: false,
            transfer_max_retries: 10,
            transfer_conflict_resolution: "overwrite".to_string(),
            transfer_skip_symlinks: true,
            transfer_halt_on_error: true,
            transfer_preserve_acl: true,
            transfer_preserve_streams: true,
            transfer_follow_symlinks: true,
            transfer_limit_bandwidth_rate: Some(2_000_000),
            transfer_auto_report: true,
            transfer_report_format: "csv".to_string(),
        };

        let serialized = toml::to_string(&original).expect("serialize");
        let deserialized: Settings = toml::from_str(&serialized).expect("deserialize");
        // Spot-check that the round trip preserved the trickiest
        // fields. A full Eq comparison is fine, but the spot checks
        // produce a more readable failure message when a field
        // is added without a serde attribute.
        assert_eq!(deserialized.theme, original.theme);
        assert_eq!(deserialized.git_log_limit, 250);
        assert_eq!(deserialized.ssh_presets.len(), 1);
        assert_eq!(deserialized.ssh_presets[0].host, "example.com");
        assert_eq!(deserialized.plugins.len(), 1);
        assert_eq!(
            deserialized.plugins.get("my-plugin").map(|p| p.trusted),
            Some(true)
        );
        assert_eq!(
            deserialized.dismissed_update_version.as_deref(),
            Some("v0.7.0")
        );
        assert_eq!(deserialized.transfer_buffer_size, 256 * 1024);
        assert_eq!(deserialized.transfer_limit_bandwidth_rate, Some(2_000_000));
    }

    #[test]
    fn settings_default_roundtrips_through_toml() {
        // The strongest test of "no required field is missing serde
        // default": take the canonical `Settings::default()`, serialise
        // it to TOML, and parse it back. If a field lacks
        // `#[serde(default)]` and isn't written by `to_string`, this
        // roundtrip will silently drop it on the parse side, so we
        // also assert that the deserialised value equals the original.
        let original = Settings::default();
        let s = toml::to_string(&original).expect("serialize default");
        let back: Settings = toml::from_str(&s).expect("parse serialised default");

        // Spot-check a handful of fields that span the struct so a
        // future missing default shows up here.
        assert_eq!(back.theme, original.theme);
        assert_eq!(back.git_log_limit, original.git_log_limit);
        assert_eq!(back.git_log_limit, 100);
        assert_eq!(back.keybinding_preset, original.keybinding_preset);
        assert_eq!(back.transfer_buffer_size, original.transfer_buffer_size);
        assert_eq!(
            back.transfer_default_hash, original.transfer_default_hash,
            "transfer_default_hash is `blake3` by default"
        );
        assert_eq!(
            back.confirmations.confirm_delete,
            original.confirmations.confirm_delete
        );
        assert_eq!(back.highlight_rules.len(), original.highlight_rules.len());
    }

    // ── PluginConfig roundtrip ──────────────────────────────────────────────

    #[test]
    fn plugin_config_roundtrips() {
        let pc = PluginConfig {
            name: "demo".to_string(),
            trusted: true,
        };
        let s = toml::to_string(&pc).expect("serialize");
        let back: PluginConfig = toml::from_str(&s).expect("deserialize");
        assert_eq!(back, pc);
    }

    #[test]
    fn plugin_config_defaults_trusted_to_false() {
        // `trusted: bool` is `#[serde(default)]`, so a missing field
        // must deserialize to `false`.
        let toml_src = r#"
name = "demo"
"#;
        let pc: PluginConfig = toml::from_str(toml_src).expect("parse");
        assert!(!pc.trusted);
        assert_eq!(pc.name, "demo");
    }

    // ── SshPreset roundtrip ────────────────────────────────────────────────

    #[test]
    fn ssh_preset_roundtrips_with_optional_fields() {
        let p = SshPreset {
            name: "lab".to_string(),
            host: "10.0.0.1".to_string(),
            port: "2222".to_string(),
            username: "alice".to_string(),
            password: Some("hunter2".to_string()),
            key_path: None,
        };
        let s = toml::to_string(&p).expect("serialize");
        let back: SshPreset = toml::from_str(&s).expect("deserialize");
        assert_eq!(back.name, p.name);
        assert_eq!(back.host, p.host);
        assert_eq!(back.password, p.password);
        assert!(back.key_path.is_none());
    }
}
