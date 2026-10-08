use crate::app::state::AppState;

/// Enters highlighted directory or open files with standard OS handlers.
pub fn handle_enter_key(state: &mut AppState, context: &crate::app::context::AppContext) {
    let mut target_dir = None;
    let mut open_file_path: Option<std::path::PathBuf> = None;
    {
        let active = state.get_active_panel();
        if let Some(entry) = active.entries.get(active.cursor_index) {
            if entry.is_dir {
                target_dir = Some(entry.path.clone());
            } else {
                if !context.config.settings.enter_use_external {
                    open_file_path = Some(entry.path.clone());
                } else {
                    let rule = crate::config::associations::AssociationsConfig::load()
                        .find_rule(&entry.name)
                        .cloned();

                    if let Some(r) = rule {
                        // Association: parse command into (program, args) and
                        // exec directly without a shell. File path is passed as
                        // a single argv entry, so a malicious filename cannot
                        // inject shell commands.
                        let (program, args) = r.resolve_open_cmd(&entry.path);
                        if !program.is_empty() {
                            if context.config.settings.automatic_update_env_variables {
                                crate::app::sys_helpers::refresh_env_vars();
                            }
                            let _ = std::process::Command::new(&program).args(&args).spawn();
                        }
                    } else if cfg!(target_os = "windows") {
                        if context.config.settings.use_windows_registered_types {
                            // No matching association: hand the path to the
                            // shell-registered handler via `start`. We route
                            // through `cmd /c` because `start` is a cmd.exe
                            // builtin, but the file path is shell-quoted to
                            // neutralise any metacharacters in the filename.
                            let path_quoted =
                                crate::app::actions::fs_ops::helper::shell_quote(&entry.path);
                            if context.config.settings.automatic_update_env_variables {
                                crate::app::sys_helpers::refresh_env_vars();
                            }
                            let _ = std::process::Command::new("cmd")
                                .arg("/c")
                                .arg(format!("start \"\" {}", path_quoted))
                                .spawn();
                        }
                    } else {
                        // No association on Unix: fall back to xdg-open. The
                        // path is passed as a separate argv entry rather than
                        // concatenated into a shell string.
                        if context.config.settings.automatic_update_env_variables {
                            crate::app::sys_helpers::refresh_env_vars();
                        }
                        let _ = std::process::Command::new("xdg-open")
                            .arg(&entry.path)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn();
                    }
                }
            }
        }
    }

    if let Some(path) = open_file_path {
        state.push_file_view_history(path.clone());
        state.open_viewer(path, context.config.settings.image_preview_enabled, false);
        return;
    }
    if let Some(dir) = target_dir {
        state.push_folders_history(dir.clone());
        let active_mut = state.get_active_panel_mut();
        // Going up via "..": land on the folder we just left.
        active_mut.pending_focus = child_name_of(&dir, &active_mut.current_path);
        active_mut.current_path = dir;
        active_mut.cursor_index = 0;
        active_mut.clear_selection();
    }
}

/// Name of the direct child of `parent` that `child` is (e.g. the folder we
/// leave when ascending), or `None` when `child` is not directly below it.
fn child_name_of(parent: &std::path::Path, child: &std::path::Path) -> Option<String> {
    (child.parent() == Some(parent))
        .then(|| child.file_name())
        .flatten()
        .map(|n| n.to_string_lossy().into_owned())
}

/// Ascends to parent folder directory.
pub fn handle_backspace_key(state: &mut AppState, show_hidden: bool) {
    let parent_path = state
        .get_active_panel()
        .current_path
        .parent()
        .map(|p| p.to_path_buf());
    if let Some(parent) = parent_path {
        state.push_folders_history(parent.clone());

        let active = state.get_active_panel_mut();
        // Reposition the cursor on the directory we just exited once the
        // background listing of the parent arrives.
        active.pending_focus = child_name_of(&parent, &active.current_path);
        active.current_path = parent;
        active.cursor_index = 0;
        active.clear_selection();

        state.refresh_active_panel(show_hidden);
    }
}
