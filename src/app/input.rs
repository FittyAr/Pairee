use super::actions::execute_shell_command;
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState};
use crate::terminal::TerminalBackend;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Captures characters for bottom shell CLI command input.
pub fn handle_cli_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<(), ()> {
    if state.active_popup.is_some() {
        return Err(());
    }

    let is_vim = context.config.keybindings.preset == "vim";
    let is_active = !state.cli_input.is_empty() || !is_vim;

    if !is_active {
        return Err(());
    }

    match key.code {
        KeyCode::Char(c) => {
            // Vim start trigger bypass
            if is_vim && state.cli_input.is_empty() && c == ':' {
                state.cli_input.push(' ');
                state.cli_input.clear();
                return Ok(());
            }

            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT {
                state.cli_input.push(c);
                return Ok(());
            }
            Err(())
        }
        KeyCode::Backspace => {
            if !state.cli_input.is_empty() {
                state.cli_input.pop();
                return Ok(());
            }
            Err(())
        }
        KeyCode::Enter => {
            if !state.cli_input.is_empty() {
                let cmd = state.cli_input.trim().to_string();
                state.cli_input.clear();
                state.push_command_history(cmd.clone());

                let current_path = state.get_active_panel().current_path.clone();

                if cmd == "cd" || cmd.starts_with("cd ") {
                    let target_dir = cmd.strip_prefix("cd").unwrap_or("").trim();
                    let new_path = if target_dir.is_empty() || target_dir == "~" {
                        let home = if cfg!(target_os = "windows") {
                            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string())
                        } else {
                            std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
                        };
                        std::path::PathBuf::from(home)
                    } else {
                        let path = std::path::Path::new(target_dir);
                        current_path.join(path)
                    };

                    let new_path = match std::fs::canonicalize(&new_path) {
                        Ok(p) => p,
                        Err(_) => new_path,
                    };

                    if new_path.is_dir() {
                        let active = state.get_active_panel_mut();
                        active.current_path = new_path;
                        active.cursor_index = 0;
                        active.selected_paths.clear();
                    }
                } else {
                    let _ = execute_shell_command(&cmd, &current_path, context, terminal_backend);
                }

                state.refresh_both_panels(context.config.settings.show_hidden);
                return Ok(());
            }
            Err(())
        }
        KeyCode::Esc => {
            if !state.cli_input.is_empty() {
                state.cli_input.clear();
                return Ok(());
            }
            Err(())
        }
        _ => Err(()),
    }
}

/// Enters highlighted directory or open files with standard OS handlers.
pub fn handle_enter_key(state: &mut AppState, context: &crate::app::context::AppContext) {
    let mut target_dir = None;
    {
        let active = state.get_active_panel();
        if let Some(entry) = active.entries.get(active.cursor_index) {
            if entry.is_dir {
                target_dir = Some(entry.path.clone());
            } else {
                let path = entry.path.to_string_lossy().to_string();
                let rule = crate::config::associations::AssociationsConfig::load()
                    .find_rule(&entry.name)
                    .cloned();

                let (cmd_to_run, should_spawn) = if let Some(r) = rule {
                    (Some(r.resolve_open_cmd(&entry.path)), true)
                } else if cfg!(target_os = "windows") {
                    if context.config.settings.use_windows_registered_types {
                        (Some(format!("start \"\" \"{}\"", path)), true)
                    } else {
                        (None, false)
                    }
                } else {
                    (Some(format!("xdg-open \"{}\" 2>/dev/null", path)), true)
                };

                if should_spawn {
                    if let Some(cmd) = cmd_to_run {
                        if context.config.settings.automatic_update_env_variables {
                            crate::app::sys_helpers::refresh_env_vars();
                        }
                        let args = if cfg!(target_os = "windows") {
                            vec!["/c", &cmd]
                        } else {
                            vec!["-c", &cmd]
                        };

                        let _ = std::process::Command::new(if cfg!(target_os = "windows") {
                            "cmd"
                        } else {
                            "sh"
                        })
                        .args(&args)
                        .spawn();
                    }
                }
            }
        }
    }
    if let Some(dir) = target_dir {
        state.push_folders_history(dir.clone());
        let active_mut = state.get_active_panel_mut();
        active_mut.current_path = dir;
        active_mut.cursor_index = 0;
        active_mut.selected_paths.clear();
    }
}

/// Ascends to parent folder directory.
pub fn handle_backspace_key(state: &mut AppState, show_hidden: bool) {
    let parent_path = state
        .get_active_panel()
        .current_path
        .parent()
        .map(|p| p.to_path_buf());
    if let Some(parent) = parent_path {
        let current_dir_name = state
            .get_active_panel()
            .current_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        state.push_folders_history(parent.clone());

        state.get_active_panel_mut().current_path = parent;
        state.get_active_panel_mut().selected_paths.clear();

        // Reread folder entries in parent directory
        state.refresh_both_panels(show_hidden);

        // Reposition cursor on directory we just exited
        let active_ref = match state.active_panel {
            ActivePanel::Left => &mut state.left_panel,
            ActivePanel::Right => &mut state.right_panel,
        };
        active_ref.cursor_index = active_ref
            .entries
            .iter()
            .position(|e| e.name == current_dir_name)
            .unwrap_or(0);
    }
}
