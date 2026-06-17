use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

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
                copy_access_mode: true, // Default as in screenshot
                copy_extended_attributes: false,
                disable_write_cache: false,
                produce_sparse_files: false,
                use_copy_on_write: false,
                symlink_mode: 0, // Smartly copy
                use_filter: false,
                filter_mask: String::new(),
            });
        } else {
            // Check overwrite if enabled
            let mut any_exists = false;
            if context.config.settings.confirmations.confirm_overwrite {
                for src in &targets {
                    if let Some(fname) = src.file_name() {
                        let dst = dest_dir.join(fname);
                        if dst.exists() {
                            any_exists = true;
                            break;
                        }
                    }
                }
            }

            if any_exists {
                state.active_popup = Some(PopupType::ConfirmOverwrite {
                    src_paths: targets,
                    dest_dir,
                    is_move: false,
                    input: None,
                });
            } else {
                let rx = crate::fs::spawn_copy_move_task(
                    targets.clone(),
                    dest_dir.clone(),
                    state.get_active_panel().ssh_conn.clone(),
                    state.get_passive_panel().ssh_conn.clone(),
                    false,
                    context.config.settings.clone(),
                );
                state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Copy {
                    sources: targets,
                    dest: dest_dir,
                });
                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    is_move: false,
                    current_file: t("progress_initializing"),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
                });
            }
        }
    }
    true
}
