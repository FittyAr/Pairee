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
