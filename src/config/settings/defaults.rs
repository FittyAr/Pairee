use super::{
    ConfirmationSettings, Settings, TabExpansion, default_plugins_dev_dir, default_ssh_timeout_secs,
};
use crate::app::state::{PanelViewMode, SortField};

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_hidden: false,
            secure_mode: false,
            mouse_support: true,
            keybinding_preset: "norton".to_string(),
            onboarding_completed: false,
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
            save_commands_history: true,
            save_folders_history: true,
            save_view_and_edit_history: true,
            use_windows_registered_types: false,
            automatic_update_env_variables: false,
            req_admin_modification: false,
            req_admin_reading: false,
            sorting_collation: "linguistic".to_string(),
            treat_digits_as_numbers: false,
            case_sensitive_sort: false,
            auto_save_setup: false,

            // Tab 1
            highlight_files: true,
            select_folders: true,
            sort_folder_names_by_extension: false,
            disable_panel_update_object_count: 0,
            show_column_titles: true,
            show_status_line: true,
            show_files_total_information: true,
            show_free_size: false,
            show_scrollbar: false,
            show_sort_mode_letter: true,
            show_dotdot_in_root_folders: false,

            // Tab 2
            interface_clock: true,
            interface_show_key_bar: true,
            interface_always_show_menu_bar: false,
            auto_drop_menu: false,
            enable_yazi_workflow: false,

            // Tab 4
            language: "English".to_string(),
            plugins_developer_mode: false,
            plugins_dev_dir: default_plugins_dev_dir(),

            // Tab 5
            enter_use_external: false,
            editor_expand_tabs: TabExpansion::Keep,
            editor_auto_indent: false,
            editor_cursor_at_end: false,
            editor_tab_size: 8,
            editor_show_line_numbers: true,
            editor_lock_editing_readonly: false,
            editor_warn_opening_readonly: false,
            viewer_use_external: false,
            viewer_tab_size: 8,
            viewer_show_scrollbar: false,

            // Tab 6
            highlight_rules: crate::ui::highlight::default_highlight_rules(),
            ssh_presets: Vec::new(),

            // Git integration
            git_enabled: true,
            ssh_enabled: true,
            ssh_timeout_secs: default_ssh_timeout_secs(),
            plugins_enabled: true,
            image_preview_enabled: true,
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
