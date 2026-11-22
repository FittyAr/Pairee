use crate::app::actions::panel_search::open as open_search;
use crate::app::context::AppContext;
use crate::app::input::cli_cd::{cd_argument, change_dir, jump, z_argument};
use crate::app::input::cli_complete::complete;
use crate::app::input::panel_find::NameMatch;
use crate::app::input::type_ahead::find_match;
use crate::app::state::AppState;
use crate::keybindings::options::TypingMode;
use crate::keybindings::screens::CliAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    context: &mut AppContext,
) -> Result<(), ()> {
    if state.dialogs.is_open() {
        return Err(());
    }
    if state.cli.is_active() {
        return edit_cli(state, key, context);
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
        TypingMode::Cli => state.cli.field.insert_char(c),
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

/// Keys while the command line is being edited: Enter runs it, Esc clears
/// it, the `[cli]` keymap completes names and deletes words, and typing and
/// the cursor keys edit the text. Other keys (Up / Down, F-keys...) go on to
/// the panels.
fn edit_cli(state: &mut AppState, key: KeyEvent, context: &mut AppContext) -> Result<(), ()> {
    match key.code {
        KeyCode::Enter if key.modifiers.is_empty() => {
            let cmd = state.cli.text().trim().to_string();
            state.cli.reset();
            if !cmd.is_empty() {
                state.push_command_history(cmd.clone());
                run_cli_command(state, &cmd, context);
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
            return Ok(());
        }
        KeyCode::Esc => {
            state.cli.reset();
            return Ok(());
        }
        _ => {}
    }
    if let Some(command) = context.resolver.cli.dispatch(key) {
        match command {
            CliAction::Complete => {
                complete(state);
            }
            CliAction::DeleteWord => {
                state.cli.completion = None;
                state.cli.field.delete_word_back();
            }
        }
        return Ok(());
    }
    // Characters only without Ctrl / Alt: those chords are shortcuts.
    if matches!(key.code, KeyCode::Char(_)) && printable(key).is_none() {
        return Err(());
    }
    if state.cli.field.handle_key(&key).consumed() {
        state.cli.completion = None;
        return Ok(());
    }
    Err(())
}

/// Runs a submitted command line: `cd`, a background `... &` job or a shell command.
fn run_cli_command(state: &mut AppState, cmd: &str, context: &AppContext) {
    if let Some(arg) = cd_argument(cmd) {
        change_dir(state, arg);
    } else if let Some(query) = z_argument(cmd) {
        jump(state, query);
    } else {
        // `command &` (the background form other shells have) runs the
        // same way: on its own screen, the panels a key away.
        let cmd = cmd.strip_suffix('&').unwrap_or(cmd);
        crate::app::actions::exec::run_in_active_panel(state, cmd, context);
    }
}
