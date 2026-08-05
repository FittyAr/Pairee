use super::actions::execute_shell_command;
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, Screen};
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

    // Bypass CLI input capture if command line is empty and the key matches a resolved shortcut
    if state.cli_input.is_empty() && context.resolver.resolve(key).is_some() {
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
                        active.clear_selection();
                    }
                } else if cmd.ends_with("&") {
                    let cmd_bg = cmd.strip_suffix("&").unwrap().trim().to_string();
                    let current_dir = current_path.clone();

                    let ts = crate::app::state::types::TerminalState {
                        command: cmd_bg.clone(),
                        output_lines: vec![],
                        is_running: true,
                        pid: None,
                    };

                    state.push_screen(crate::app::state::Screen::Terminal(ts));
                    let screen_idx = state.screens.len() - 1;
                    let tx = state.term_tx.clone();

                    let shell = if cfg!(target_os = "windows") {
                        "cmd"
                    } else {
                        "sh"
                    };
                    let arg = if cfg!(target_os = "windows") {
                        "/c"
                    } else {
                        "-c"
                    };

                    tokio::spawn(async move {
                        use std::process::Stdio;
                        use tokio::io::AsyncBufReadExt;

                        let mut child = match tokio::process::Command::new(shell)
                            .arg(arg)
                            .arg(&cmd_bg)
                            .current_dir(current_dir)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .spawn()
                        {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = tx.send(crate::app::state::TerminalUpdate {
                                    screen_idx,
                                    line: Some(format!("Failed to spawn: {}", e)),
                                });
                                let _ = tx.send(crate::app::state::TerminalUpdate {
                                    screen_idx,
                                    line: None,
                                });
                                return;
                            }
                        };

                        // `Command` is configured with `Stdio::piped()` for
                        // both handles above, so `take()` must succeed. We
                        // still `match` instead of `unwrap` so that a
                        // future refactor that drops the `.stdout(...)`
                        // call cannot panic the spawned task and crash
                        // the whole terminal screen.
                        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
                            let _ = tx.send(crate::app::state::TerminalUpdate {
                                screen_idx,
                                line: Some(
                                    "Internal error: child stdio handles not piped".to_string(),
                                ),
                            });
                            let _ = tx.send(crate::app::state::TerminalUpdate {
                                screen_idx,
                                line: None,
                            });
                            return;
                        };

                        let tx_out = tx.clone();
                        let tx_err = tx.clone();

                        let mut out_reader = tokio::io::BufReader::new(stdout).lines();
                        let mut err_reader = tokio::io::BufReader::new(stderr).lines();

                        let out_task = tokio::spawn(async move {
                            while let Ok(Some(line)) = out_reader.next_line().await {
                                let _ = tx_out.send(crate::app::state::TerminalUpdate {
                                    screen_idx,
                                    line: Some(line),
                                });
                            }
                        });

                        let err_task = tokio::spawn(async move {
                            while let Ok(Some(line)) = err_reader.next_line().await {
                                let _ = tx_err.send(crate::app::state::TerminalUpdate {
                                    screen_idx,
                                    line: Some(line),
                                });
                            }
                        });

                        let _ = tokio::join!(out_task, err_task);
                        let _ = child.wait().await;

                        let _ = tx.send(crate::app::state::TerminalUpdate {
                            screen_idx,
                            line: None,
                        });
                    });
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
                            let _ = std::process::Command::new(&program)
                                .args(&args)
                                .spawn();
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
        let viewer = crate::ui::viewer::ViewerState::load(path);
        state.push_screen(Screen::Viewer(viewer));
        return;
    }
    if let Some(dir) = target_dir {
        state.push_folders_history(dir.clone());
        let active_mut = state.get_active_panel_mut();
        active_mut.current_path = dir;
        active_mut.cursor_index = 0;
        active_mut.clear_selection();
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
        state.get_active_panel_mut().clear_selection();

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
