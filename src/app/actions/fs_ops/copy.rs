use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::{submit_simple, transfer_options_from_settings};

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let dest_dir = state.get_passive_panel().current_path.clone();
        if context.config.settings.confirmations.confirm_copy {
            let default_input = if targets.len() == 1 {
                targets
                    .first()
                    .and_then(|p| p.file_name())
                    .map(|n| dest_dir.join(n).to_string_lossy().to_string())
                    .unwrap_or_else(|| dest_dir.to_string_lossy().to_string())
            } else {
                dest_dir.to_string_lossy().to_string()
            };
            state.active_popup = Some(PopupType::CopyPrompt {
                input: default_input,
                src_paths: targets,
                dest_dir,
                cursor_idx: 0,
                already_existing: 0, // Ask
                process_multiple: false,
                copy_access_mode: true,
                copy_extended_attributes: false,
                disable_write_cache: false,
                produce_sparse_files: false,
                use_copy_on_write: false,
                symlink_mode: 0,
                use_filter: false,
                filter_mask: String::new(),
            });
        } else {
            let options = transfer_options_from_settings(&context.config.settings);
            submit_simple(
                state,
                TransferOperation::Copy,
                targets,
                dest_dir,
                options,
                state.get_active_panel().ssh_conn.clone(),
                state.get_passive_panel().ssh_conn.clone(),
            );
        }
    }
    true
}
