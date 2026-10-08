use serde::{Deserialize, Serialize};

/// Confirmation settings — which operations require an explicit confirmation dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationSettings {
    pub confirm_delete: bool,
    pub confirm_wipe: bool,
    pub confirm_quit: bool,
    pub confirm_copy: bool,
    pub confirm_move: bool,
    pub confirm_delete_non_empty_folders: bool,
    pub confirm_interrupt_operation: bool,
    pub confirm_reload_edited_file: bool,
    pub confirm_clear_history_list: bool,
}

impl Default for ConfirmationSettings {
    fn default() -> Self {
        Self {
            confirm_delete: true,
            confirm_wipe: true,
            confirm_quit: false,
            confirm_copy: true,
            confirm_move: true,
            confirm_delete_non_empty_folders: true,
            confirm_interrupt_operation: true,
            confirm_reload_edited_file: true,
            confirm_clear_history_list: true,
        }
    }
}
