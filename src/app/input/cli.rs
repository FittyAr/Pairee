use crate::app::actions::execute_shell_command;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::terminal::TerminalBackend;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Captures characters for bottom shell CLI command input.
pub fn handle_cli_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<(), ()> {
    if state.dialogs.is_some() {
        return Err(());
    }

    // Bypass CLI capture when this key is (or starts) a bound shortcut.
    // Uses immutable peek so multi-key sequences are not consumed here.
    if state.cli_input.is_empty() && context.resolver.would_trigger(key) {
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
                        job_id: None,
                    };

                    state.push_screen(crate::app::state::Screen::Terminal(ts));
                    let screen_idx = state.screens.len() - 1;
                    let tx = state.term_tx.clone();

                    tokio::task::spawn_blocking(move || {
                        let tx_line = tx.clone();
                        let run = crate::terminal::pty_cmd::stream_shell_on_pty(
                            &cmd_bg,
                            Some(&current_dir),
                            |line| {
                                let _ = tx_line.send(crate::app::state::TerminalUpdate {
                                    screen_idx,
                                    line: Some(line),
                                });
                            },
                        );
                        if let Err(e) = run {
                            let _ = tx.send(crate::app::state::TerminalUpdate {
                                screen_idx,
                                line: Some(format!("Failed to spawn: {e}")),
                            });
                        }
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
