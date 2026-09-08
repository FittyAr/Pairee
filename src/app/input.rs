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

/// First non-empty line of a bracketed-paste payload (`\r` stripped).
pub fn first_paste_line(raw: &str) -> String {
    raw.replace('\r', "")
        .lines()
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Insert a paste into the focused text field, or the CLI when no overlay is open.
pub fn handle_paste(state: &mut AppState, raw: &str) {
    let line = first_paste_line(raw);
    if line.is_empty() {
        return;
    }
    if state.dialogs.is_some() {
        if let Some(popup) = state.dialogs.top_mut() {
            let _ = popup.apply_paste(&line);
        }
        return;
    }
    state.cli_input.push_str(&line);
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
        let viewer = crate::ui::viewer::ViewerState::load_with_images(
            path,
            context.config.settings.image_preview_enabled,
        );
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
        let active_ref = match state.panels.active {
            ActivePanel::Left => &mut state.panels.left,
            ActivePanel::Right => &mut state.panels.right,
        };
        active_ref.cursor_index = active_ref
            .entries
            .iter()
            .position(|e| e.name == current_dir_name)
            .unwrap_or(0);
    }
}

#[cfg(test)]
mod paste_tests {
    use super::*;
    use crate::app::state::PopupType;
    use std::path::PathBuf;

    #[test]
    fn first_paste_line_strips_cr_and_extra_lines() {
        assert_eq!(first_paste_line("hello\r\nworld"), "hello");
        assert_eq!(first_paste_line("\n\nfoo"), "foo");
        assert_eq!(first_paste_line(""), "");
    }

    #[test]
    fn paste_goes_to_cli_when_no_dialog() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        handle_paste(&mut state, "echo hi\r\nignored");
        assert_eq!(state.cli_input, "echo hi");
    }

    #[test]
    fn paste_goes_to_rename_prompt() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::RenamePrompt {
            input: "old".into(),
            original: "old".into(),
            src_path: PathBuf::from("old"),
            parent_dir: PathBuf::from("."),
            cursor_idx: 0,
        });
        handle_paste(&mut state, "new-name.txt\n");
        match state.dialogs.top() {
            Some(PopupType::RenamePrompt { input, .. }) => {
                assert_eq!(input, "oldnew-name.txt");
            }
            other => panic!("expected rename prompt, got {other:?}"),
        }
        assert!(state.cli_input.is_empty());
    }

    #[test]
    fn paste_ignored_on_non_text_dialog() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::ConfirmQuit);
        handle_paste(&mut state, "should-not-land-in-cli");
        assert!(state.cli_input.is_empty());
    }

    #[test]
    fn paste_goes_to_apply_command() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::ApplyCommandPrompt {
            input: "echo ".into(),
            targets: vec![],
        });
        handle_paste(&mut state, "%f\r\nmore");
        match state.dialogs.top() {
            Some(PopupType::ApplyCommandPrompt { input, .. }) => {
                assert_eq!(input, "echo %f");
            }
            other => panic!("expected apply prompt, got {other:?}"),
        }
    }
}
