//! Keys of the duplicate finder: the options form (smallest size, hash;
//! Enter starts the search) and the results (marks, delete / trash, go to
//! a file).

use crate::app::context::AppContext;
use crate::app::form::FormKey;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::forms::DUPE_OPTIONS_FORM;
use crate::app::state::popup::{Dialog, DupeOptionsPrompt, DuplicatesView, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::fs::dupes::DupeOptions;
use crate::fs::dupes::keep::Keep;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::str::FromStr;

/// Rows of the hash algorithm and the follow-links checkbox in the
/// options form.
const ROW_ALGORITHM: usize = 1;
const ROW_FOLLOW_LINKS: usize = 2;
const BUTTON_CANCEL: usize = 4;

fn handle_options(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(prompt) = state.dialogs.top_mut_as::<DupeOptionsPrompt>() else {
        return Err(());
    };
    let field = (prompt.cursor_idx == 0).then_some(&mut prompt.min_size);
    match DUPE_OPTIONS_FORM.handle(&mut prompt.cursor_idx, field, &key) {
        FormKey::Toggle(ROW_ALGORITHM) => prompt.algorithm = prompt.algorithm.next(),
        FormKey::Toggle(ROW_FOLLOW_LINKS) => prompt.follow_links = !prompt.follow_links,
        FormKey::Other
            if prompt.cursor_idx == ROW_ALGORITHM
                && matches!(key.code, KeyCode::Left | KeyCode::Right) =>
        {
            prompt.algorithm = prompt.algorithm.next();
        }
        FormKey::Activate(BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(_) => start(state),
        FormKey::Toggle(_) | FormKey::Handled | FormKey::Other => {}
    }
    Ok(None)
}

/// Starts the search the options describe and shows its results.
fn start(state: &mut AppState) {
    let Some(prompt) = state.dialogs.top_as::<DupeOptionsPrompt>() else {
        return;
    };
    let Some(min_size) = parse_size(prompt.min_size.text()) else {
        let text = t("dupes_invalid_size").replace("{}", prompt.min_size.text());
        state.dialogs.push(PopupType::error(text));
        return;
    };
    let options = DupeOptions {
        min_size,
        algorithm: prompt.algorithm,
        follow_links: prompt.follow_links,
        ..DupeOptions::default()
    };
    let roots = prompt.roots.clone();
    let source = state.get_active_panel().source.clone();
    state.dupes.start(roots, source, options);
    state.dialogs.replace(DuplicatesView);
}

/// `"1"`, `"4 KiB"`, `"10MB"`... in bytes (empty: 1 byte).
fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() {
        return Some(1);
    }
    text.parse::<u64>()
        .ok()
        .or_else(|| bytesize::ByteSize::from_str(text).ok().map(|s| s.as_u64()))
}

fn handle_results(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let dupes = &mut state.dupes;
    let len = dupes.file_count();
    match list_key(ListKeys::FULL, key.code, &mut dupes.cursor, len) {
        ListKey::Moved => {}
        ListKey::Activate(_) => go_to_file(state, context),
        ListKey::Close => close(state),
        ListKey::Other => match key.code {
            KeyCode::Char(' ') | KeyCode::Insert => dupes.toggle_selected(),
            KeyCode::Char('n' | 'N') => dupes.mark_all_but(Keep::Newest),
            KeyCode::Char('o' | 'O') => dupes.mark_all_but(Keep::Oldest),
            KeyCode::Char('f' | 'F') => dupes.mark_all_but(Keep::First),
            KeyCode::Char('u' | 'U') => dupes.unmark_all(),
            KeyCode::Delete | KeyCode::F(8) => delete_marked(state, context, false),
            KeyCode::Char('t' | 'T') => delete_marked(state, context, true),
            KeyCode::F(10) => close(state),
            _ => {}
        },
    }
    Ok(None)
}

/// Stops a running search and closes the results.
fn close(state: &mut AppState) {
    state.dupes.cancel();
    state.dialogs.pop();
}

/// Shows the highlighted file in the active panel.
fn go_to_file(state: &mut AppState, context: &AppContext) {
    let Some(path) = state.dupes.selected().cloned() else {
        return;
    };
    state.dialogs.clear();
    state.get_active_panel_mut().reveal(&path);
    state.refresh_both_panels(context.config.settings.show_hidden);
}

/// Sends the marked files to the regular delete flow (confirmation,
/// transfer queue); the view drops them when the delete job finishes.
fn delete_marked(state: &mut AppState, context: &AppContext, trash: bool) {
    if state.dupes.is_scanning() {
        return;
    }
    use crate::app::actions::fs_ops::delete::{DeleteMode, request};
    let mode = if trash {
        DeleteMode::Trash
    } else {
        DeleteMode::Configured
    };
    let marked = state.dupes.marked();
    request(state, context, marked, true, mode);
}

impl Dialog for DupeOptionsPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle_options
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        (self.cursor_idx == 0).then_some(&mut self.min_size)
    }
}

impl Dialog for DuplicatesView {
    fn key_handler(&self) -> KeyHandler {
        handle_results
    }
    fn resizable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_plain_numbers_and_units() {
        assert_eq!(parse_size(""), Some(1));
        assert_eq!(parse_size("0"), Some(0));
        assert_eq!(parse_size(" 512 "), Some(512));
        assert_eq!(parse_size("4 KiB"), Some(4096));
        assert_eq!(parse_size("1MB"), Some(1_000_000));
        assert_eq!(parse_size("lots"), None);
    }
}
