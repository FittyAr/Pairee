use crate::app::context::AppContext;
use crate::app::state::types::EditorState;
use crate::app::state::{AppState, FileAttrsSnapshot, LinkKind, PopupType, Screen};
use crate::keybindings::Action;
use crate::terminal::TerminalBackend;

fn is_non_empty_dir(path: &std::path::Path) -> bool {
    if path.is_dir() {
        if let Ok(mut entries) = std::fs::read_dir(path) {
            entries.next().is_some()
        } else {
            false
        }
    } else {
        false
    }
}

/// Handles filesystem-related actions. Returns `true` if the action was handled.
pub fn handle_fs_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
) -> bool {
    match action {
        Action::View | Action::ViewAlt => {
            let active = state.get_active_panel();
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
                let path = entry.path.clone();
                let entry_name = entry.name.clone();
                state.push_file_view_history(path.clone());

                let rule = crate::config::associations::AssociationsConfig::load()
                    .find_rule(&entry_name)
                    .cloned();

                let mut ran_external = false;

                // Decide whether we want to use the external command.
                // If viewer_use_external is true:
                //   F3 (Action::View) uses external command.
                //   Alt+F3 (Action::ViewAlt) uses internal viewer.
                // If viewer_use_external is false (default):
                //   F3 (Action::View) uses internal viewer.
                //   Alt+F3 (Action::ViewAlt) uses external command.
                let use_external = match action {
                    Action::View => context.config.settings.viewer_use_external,
                    Action::ViewAlt => !context.config.settings.viewer_use_external,
                    _ => false,
                };

                if use_external {
                    if let Some(ref r) = rule {
                        let cmd = r.resolve_view_cmd(&path);
                        if command_exists(&cmd) {
                            ran_external = true;
                            if let Err(e) = super::exec::execute_external_command(
                                &path,
                                &cmd,
                                context,
                                terminal_backend,
                            ) {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Failed to run viewer: {}", e)));
                            }
                        }
                    }
                }

                if !ran_external {
                    let viewer = crate::ui::viewer::ViewerState::load(path);
                    state.push_screen(Screen::Viewer(viewer));
                }
            }
            true
        }
        Action::Edit => {
            let active = state.get_active_panel();
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
                let path = entry.path.clone();
                state.push_file_view_history(path.clone());
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        state.push_screen(Screen::Editor(EditorState {
                            path,
                            lines: if lines.is_empty() {
                                vec![String::new()]
                            } else {
                                lines
                            },
                            cursor_x: 0,
                            cursor_y: 0,
                            scroll_y: 0,
                            is_dirty: false,
                            last_search: None,
                            last_case_sensitive: false,
                        }));
                    }
                    Err(e) => {
                        state.active_popup =
                            Some(PopupType::Error(format!("Cannot read file: {}", e)));
                    }
                }
            }
            true
        }
        Action::Copy => {
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
                        let rx = crate::fs::spawn_copy_task(
                            targets.clone(),
                            dest_dir.clone(),
                            context.config.settings.clone(),
                        );
                        state.active_bg_op = Some(crate::app::state::BackgroundOpContext::Copy {
                            sources: targets,
                            dest: dest_dir,
                        });
                        state.progress_rx = Some(rx);
                        state.active_popup = Some(PopupType::CopyProgress {
                            current_file: "Initializing...".to_string(),
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
        Action::Move => {
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
                        // Move directly
                        let mut succeeded = true;
                        for src in &targets {
                            if let Some(fname) = src.file_name() {
                                let dst = dest_dir.join(fname);
                                if let Err(e) = crate::fs::rename_or_move_sync(
                                    src,
                                    &dst,
                                    context.config.settings.req_admin_modification,
                                ) {
                                    succeeded = false;
                                    if !context.config.settings.req_admin_modification {
                                        state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                                            paths: targets.clone(),
                                            op_kind: crate::app::state::AdminOpKind::RenameMove { dst: dest_dir.clone() },
                                        });
                                    } else {
                                        state.active_popup =
                                            Some(PopupType::Error(format!("Move failed: {}", e)));
                                    }
                                    break;
                                }
                            }
                        }
                        if succeeded && context.config.settings.req_admin_modification {
                            state.terminal_needs_clear = true;
                        }
                        state.get_active_panel_mut().selected_paths.clear();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                    }
                }
            }
            true
        }
        Action::CompressFiles => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let default_name = targets
                    .first()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "archive".to_string());
                let dest_dir = state.get_passive_panel().current_path.clone();
                state.active_popup = Some(PopupType::CompressPrompt {
                    input: default_name,
                    targets,
                    dest_dir,
                });
            }
            true
        }
        Action::ExtractArchive => {
            let active = state.get_active_panel();
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
                let dest = state.get_passive_panel().current_path.clone();
                let rx = crate::fs::spawn_extract_task(entry.path.clone(), dest);
                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    current_file: "Extracting...".to_string(),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
                });
            }
            true
        }
        Action::MkDir => {
            state.active_popup = Some(PopupType::MkDirPrompt {
                input: String::new(),
                cursor_idx: 0,
                process_multiple: false,
            });
            true
        }
        Action::Delete => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let show_prompt = context.config.settings.confirmations.confirm_delete
                    || (context
                        .config
                        .settings
                        .confirmations
                        .confirm_delete_non_empty_folders
                        && targets.iter().any(|p| is_non_empty_dir(p)));

                if show_prompt {
                    state.active_popup = Some(PopupType::ConfirmDelete {
                        paths: targets,
                        cursor_idx: 0,
                    });
                } else {
                    for path in &targets {
                        if let Err(e) = crate::fs::delete_sync(
                            path,
                            context.config.settings.delete_to_recycle_bin,
                            context.config.settings.req_admin_modification,
                        ) {
                            if !context.config.settings.req_admin_modification {
                                state.active_popup = Some(PopupType::ConfirmRetryAsAdmin {
                                    paths: targets.clone(),
                                    op_kind: crate::app::state::AdminOpKind::Delete,
                                });
                            } else {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Delete failed: {}", e)));
                            }
                            return true;
                        }
                    }
                    if context.config.settings.req_admin_modification {
                        state.terminal_needs_clear = true;
                    }
                    state.get_active_panel_mut().selected_paths.clear();
                    state.refresh_both_panels(context.config.settings.show_hidden);
                }
            }
            true
        }
        Action::WipeFile => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                if context.config.settings.confirmations.confirm_wipe {
                    state.active_popup = Some(PopupType::WipeConfirm { paths: targets });
                } else {
                    let rx = crate::fs::spawn_wipe_task(targets);
                    state.progress_rx = Some(rx);
                    state.active_popup = Some(PopupType::CopyProgress {
                        current_file: "Wiping...".to_string(),
                        files_copied: 0,
                        total_files: 0,
                        bytes_copied: 0,
                        total_bytes: 0,
                    });
                }
            }
            true
        }
        Action::CreateLink => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index) {
                if entry.name != ".." {
                    state.active_popup = Some(PopupType::CreateLinkPrompt {
                        src: entry.path.clone(),
                        dest_input: entry.name.clone(),
                        kind: LinkKind::Symbolic,
                    });
                }
            }
            true
        }
        Action::FileAttributes => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index) {
                if entry.name != ".." {
                    match crate::fs::read_attrs(&entry.path) {
                        Ok(attrs) => {
                            let mode_octal = format!("{:o}", attrs.mode & 0o7777);
                            state.active_popup = Some(PopupType::FileAttributesDialog {
                                attrs: FileAttrsSnapshot {
                                    path: attrs.path,
                                    readonly: attrs.readonly,
                                    size: attrs.size,
                                    modified: attrs.modified,
                                    created: attrs.created,
                                    owner: attrs.owner,
                                    nlinks: attrs.nlinks,
                                },
                                mode_input: mode_octal,
                            });
                        }
                        Err(e) => {
                            state.active_popup =
                                Some(PopupType::Error(format!("Cannot read attrs: {}", e)));
                        }
                    }
                }
            }
            true
        }
        Action::ApplyCommand => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                state.active_popup = Some(PopupType::ApplyCommandPrompt {
                    input: String::new(),
                    targets,
                });
            }
            true
        }
        Action::DescribeFile => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index) {
                if entry.name != ".." {
                    let current_desc =
                        crate::fs::read_description(&active.current_path.clone(), &entry.name)
                            .unwrap_or_default();
                    state.active_popup = Some(PopupType::DescribeFilePrompt {
                        path: entry.path.clone(),
                        current_desc: current_desc.clone(),
                        input: current_desc,
                    });
                }
            }
            true
        }
        Action::ArchiveCommands => {
            let active = state.get_active_panel();
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
                state.active_popup = Some(PopupType::ArchiveCommandsMenu {
                    archive_path: entry.path.clone(),
                    items: vec![
                        "1. List contents".to_string(),
                        "2. Test integrity".to_string(),
                        "3. Extract here".to_string(),
                        "4. Extract to other panel".to_string(),
                    ],
                    cursor_idx: 0,
                });
            }
            true
        }
        _ => false,
    }
}

fn command_exists(cmd: &str) -> bool {
    let cmd_name = match cmd.split_whitespace().next() {
        Some(name) => name,
        None => return false,
    };

    let path = std::path::Path::new(cmd_name);
    if path.is_absolute() || path.exists() {
        return true;
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for p in std::env::split_paths(&path_env) {
            let full_path = p.join(cmd_name);
            if full_path.exists() {
                return true;
            }
            if cfg!(target_os = "windows") {
                for ext in &["exe", "bat", "cmd", "com"] {
                    if full_path.with_extension(ext).exists() {
                        return true;
                    }
                }
            }
        }
    }
    false
}
