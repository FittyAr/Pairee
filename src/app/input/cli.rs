use crate::app::actions::execute_shell_command;
use crate::app::actions::panel_search::open as open_search;
use crate::app::context::AppContext;
use crate::app::input::panel_find::NameMatch;
use crate::app::input::type_ahead::find_match;
use crate::app::state::AppState;
use crate::keybindings::options::TypingMode;
use crate::terminal::TerminalBackend;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Captures characters for bottom shell CLI command input.
///
/// While the command line is idle (empty and not focused), bound keys go to
/// the keymap and an unbound printable key follows the preset's
/// [`TypingMode`]: it starts the command line (Far), jumps to a file
/// (type-ahead) or is left to the keymap and plugins (Vim / yazi).
pub fn handle_cli_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<(), ()> {
    if state.dialogs.is_some() {
        return Err(());
    }
    if state.cli_focused || !state.cli_input.is_empty() {
        return edit_cli(state, key, context, terminal_backend);
    }
    // Uses an immutable peek so multi-key sequences are not consumed here.
    if context.resolver.would_trigger(key) {
        return Err(());
    }
    if let Some(c) = alt_letter(key)
        && context.resolver.options().alt_quick_search
    {
        open_search(state, NameMatch::Prefix, Some(c));
        return Ok(());
    }
    let Some(c) = printable(key) else {
        return Err(());
    };
    match context.resolver.options().typing {
        TypingMode::Cli => state.cli_input.push(c),
        TypingMode::TypeAhead => type_ahead(state, c),
        TypingMode::Commands => return Err(()),
    }
    Ok(())
}

/// A character typed without modifiers other than Shift.
fn printable(key: KeyEvent) -> Option<char> {
    match key.code {
        KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            Some(c)
        }
        _ => None,
    }
}

/// A character typed with Alt (and maybe Shift).
fn alt_letter(key: KeyEvent) -> Option<char> {
    let alt = key.modifiers == KeyModifiers::ALT
        || key.modifiers == KeyModifiers::ALT | KeyModifiers::SHIFT;
    match key.code {
        KeyCode::Char(c) if alt && !c.is_control() => Some(c),
        _ => None,
    }
}

/// Moves the active panel's cursor to the entry the typed letters name.
fn type_ahead(state: &mut AppState, c: char) {
    let query = state.type_ahead.push(c, Instant::now()).to_string();
    let panel = state.get_active_panel_mut();
    let names: Vec<&str> = panel.entries.iter().map(|e| e.name.as_str()).collect();
    if let Some(i) = find_match(&names, panel.cursor_index, &query) {
        panel.cursor_index = i;
    }
}

/// Keys while the command line is being edited.
fn edit_cli(
    state: &mut AppState,
    key: KeyEvent,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<(), ()> {
    if let Some(c) = printable(key) {
        state.cli_input.push(c);
        return Ok(());
    }
    match key.code {
        KeyCode::Backspace => {
            state.cli_input.pop();
            Ok(())
        }
        KeyCode::Enter => {
            state.cli_focused = false;
            state.cli_history_pos = None;
            let cmd = state.cli_input.trim().to_string();
            state.cli_input.clear();
            if !cmd.is_empty() {
                state.push_command_history(cmd.clone());
                run_cli_command(state, &cmd, context, terminal_backend);
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
            Ok(())
        }
        KeyCode::Esc => {
            state.cli_focused = false;
            state.cli_history_pos = None;
            state.cli_input.clear();
            Ok(())
        }
        _ => Err(()),
    }
}

/// Runs a submitted command line: `cd`, a background `... &` job or a shell command.
fn run_cli_command(
    state: &mut AppState,
    cmd: &str,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) {
    let current_path = state.get_active_panel().current_path.clone();
    if cmd == "cd" || cmd.starts_with("cd ") {
        change_dir(
            state,
            cmd.strip_prefix("cd").unwrap_or("").trim(),
            &current_path,
        );
    } else if let Some(cmd_bg) = cmd.strip_suffix('&') {
        spawn_background(state, cmd_bg.trim().to_string(), current_path);
    } else {
        let _ = execute_shell_command(cmd, &current_path, context, terminal_backend);
    }
}

/// `cd [dir]`: an empty target or `~` goes home; relative targets resolve
/// against the active panel's folder.
fn change_dir(state: &mut AppState, target_dir: &str, current_path: &Path) {
    let new_path = if target_dir.is_empty() || target_dir == "~" {
        crate::app::session::restore::home_dir()
    } else {
        current_path.join(Path::new(target_dir))
    };
    let new_path = std::fs::canonicalize(&new_path).unwrap_or(new_path);
    if new_path.is_dir() {
        let active = state.get_active_panel_mut();
        active.current_path = new_path;
        active.cursor_index = 0;
        active.clear_selection();
    }
}

/// `command &`: runs on a PTY in a new terminal screen, streaming its output.
fn spawn_background(state: &mut AppState, cmd_bg: String, current_dir: PathBuf) {
    use crate::app::state::TerminalUpdate;
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
        let run =
            crate::terminal::pty_cmd::stream_shell_on_pty(&cmd_bg, Some(&current_dir), |line| {
                let _ = tx_line.send(TerminalUpdate {
                    screen_idx,
                    line: Some(line),
                });
            });
        if let Err(e) = run {
            let _ = tx.send(TerminalUpdate {
                screen_idx,
                line: Some(format!("Failed to spawn: {e}")),
            });
        }
        let _ = tx.send(TerminalUpdate {
            screen_idx,
            line: None,
        });
    });
}
