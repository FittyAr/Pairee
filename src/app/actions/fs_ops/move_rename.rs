use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;

pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let targets = state.get_active_panel().get_targeted_paths();
    if !targets.is_empty() {
        let dest_dir = state.get_passive_panel().current_path.clone();
        if context.config.settings.confirmations.confirm_move {
            let default_input = if targets.len() == 1 {
                targets
                    .first()
                    .and_then(|p| p.file_name())
                    .map(|n| dest_dir.join(n).to_string_lossy().to_string())
                    .unwrap_or_else(|| dest_dir.to_string_lossy().to_string())
            } else {
                dest_dir.to_string_lossy().to_string()
            };
            state.active_popup = Some(PopupType::RenMovPrompt {
                input: default_input,
                src_paths: targets,
                dest_dir,
                cursor_idx: 0,
                already_existing: 0,
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
                    is_move: true,
                    input: None,
                });
            } else {
                if context.config.settings.transfer_engine_enabled {
                    use crate::fs::transfer::engine::TransferEngine;
                    use crate::fs::transfer::job::{TransferJob, TransferOperation};
                    use crate::fs::transfer::options::TransferOptions;

                    let mut options = TransferOptions::default();
                    options.verify_after_copy = context.config.settings.transfer_verify_after_copy;
                    options.hash_algorithm = match context.config.settings.transfer_default_hash.as_str() {
                        "crc32" => crate::fs::transfer::options::HashAlgorithm::Crc32,
                        "md5" => crate::fs::transfer::options::HashAlgorithm::Md5,
                        "sha1" => crate::fs::transfer::options::HashAlgorithm::Sha1,
                        "sha256" => crate::fs::transfer::options::HashAlgorithm::Sha256,
                        _ => crate::fs::transfer::options::HashAlgorithm::Blake3,
                    };
                    options.buffer_size = match context.config.settings.transfer_buffer_size {
                        65536 => crate::fs::transfer::options::BufferSize::_64KB,
                        262144 => crate::fs::transfer::options::BufferSize::_256KB,
                        4194304 => crate::fs::transfer::options::BufferSize::_4MB,
                        _ => crate::fs::transfer::options::BufferSize::_1MB,
                    };
                    options.direct_io = context.config.settings.transfer_direct_io;
                    options.preserve_timestamps = context.config.settings.transfer_preserve_timestamps;
                    options.preserve_attributes = context.config.settings.transfer_preserve_attributes;
                    options.preserve_acl = context.config.settings.transfer_preserve_acl;
                    options.preserve_streams = context.config.settings.transfer_preserve_streams;
                    options.skip_symlinks = context.config.settings.transfer_skip_symlinks;
                    options.follow_symlinks = context.config.settings.transfer_follow_symlinks;
                    options.limit_bandwidth_rate = context.config.settings.transfer_limit_bandwidth_rate;
                    options.max_retries = context.config.settings.transfer_max_retries;
                    options.conflict_resolution = context.config.settings.transfer_conflict_resolution.clone();

                    let job = TransferJob::new(
                        TransferOperation::Move,
                        targets.clone(),
                        dest_dir.clone(),
                        options,
                    );

                    if state.transfer.is_none() {
                        let (engine, rx) = TransferEngine::new();
                        state.transfer = Some(crate::app::state::transfer_state::TransferUIState::new(engine, rx));
                    }

                    if let Some(ref mut ts) = state.transfer {
                        ts.engine.submit_job(job);
                        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                    }
                    state.active_popup = None;
                } else {
                    let rx = crate::fs::spawn_copy_move_task(
                        targets.clone(),
                        dest_dir.clone(),
                        state.get_active_panel().ssh_conn.clone(),
                        state.get_passive_panel().ssh_conn.clone(),
                        true,
                        context.config.settings.clone(),
                    );
                    state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Move {
                        sources: targets,
                        dest: dest_dir,
                    });
                    state.progress_rx = Some(rx);
                    state.active_popup = Some(PopupType::CopyProgress {
                        is_move: true,
                        current_file: t("progress_initializing"),
                        files_copied: 0,
                        total_files: 0,
                        bytes_copied: 0,
                        total_bytes: 0,
                    });
                }
            }
        }
    }
    true
}
