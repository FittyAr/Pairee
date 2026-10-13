//! Popups of the built-in editor: find / replace, "save as" and overwrite
//! confirmation.

use crate::app::context::AppContext;
use crate::app::editor::open::{resolve_save_as_path, save_active_editor};
use crate::app::form::{FieldKey, confirm_answer, field_key};
use crate::app::screen_input::editor::{editor_page_height, report_search};
use crate::app::state::popup::EditorConfirmOverwrite;
use crate::app::state::popup::EditorSaveAsPrompt;
use crate::app::state::popup::{Dialog, EditorSearchPrompt, FindButton, FindKey, KeyHandler};
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::localization::t;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top_mut() {
        Some(PopupType::EditorSearchPrompt(search)) => {
            if let Some(older) = EditorSearchPrompt::history_step(&key) {
                let history = &state.history;
                search.recall(&history.searches, &history.replacements, older);
                return Ok(None);
            }
            match search.handle_key(&key) {
                FindKey::Stay => {}
                FindKey::Close => state.dialogs.clear(),
                FindKey::Run(button) => run_search(state, button),
            }
            Ok(None)
        }
        Some(PopupType::EditorSaveAsPrompt(EditorSaveAsPrompt { input })) => {
            match field_key(input, &key) {
                FieldKey::Cancel => state.dialogs.clear(),
                FieldKey::Submit => {
                    let input = input.text().to_string();
                    save_as(state, &input);
                }
                FieldKey::Handled | FieldKey::Other => {}
            }
            Ok(None)
        }
        Some(PopupType::EditorConfirmOverwrite(EditorConfirmOverwrite { target, .. })) => {
            let target = target.clone();
            match confirm_answer(&key, true) {
                Some(true) => {
                    state.dialogs.clear();
                    save_active_editor(state, Some(target), true);
                }
                Some(false) => state.dialogs.clear(),
                None => {}
            }
            Ok(None)
        }
        _ => Err(()),
    }
}

/// A button of the find / replace dialog. [Find] and [All] close it,
/// [Replace] keeps it open for the next match.
fn run_search(state: &mut AppState, button: FindButton) {
    let Some(PopupType::EditorSearchPrompt(dialog)) = state.dialogs.top() else {
        return;
    };
    let search = match dialog.to_search() {
        Ok(Some(search)) => search,
        Ok(None) => return state.dialogs.clear(),
        Err(e) => {
            let text = t("editor_find_bad_regex").replacen("{}", &e.to_string(), 1);
            return state.dialogs.push(PopupType::error(text));
        }
    };
    state
        .history
        .push_search(search.pattern.clone(), search.replacement.clone());
    if button != FindButton::Replace {
        state.dialogs.clear();
    }
    let height = editor_page_height();
    let Some(ed) = state.active_editor_mut() else {
        return state.dialogs.clear();
    };
    ed.show_matches = true;
    ed.last_search = Some(Box::new(search.clone()));
    let backward = search.options.backward;
    let outcome = match button {
        FindButton::Replace => ed.replace_match(&search, backward, height),
        FindButton::ReplaceAll => {
            let message = match ed.replace_all(&search) {
                Some(n) => t("editor_replaced_count").replacen("{}", &n.to_string(), 1),
                None => t("editor_read_only_locked"),
            };
            return state.dialogs.replace(PopupType::info(message));
        }
        FindButton::Find | FindButton::Cancel => ed.find_match(&search, backward, height),
    };
    report_search(state, outcome);
}

/// Enter in "save as": save to the typed path (relative to the file's folder).
fn save_as(state: &mut AppState, input: &str) {
    let target = state
        .active_editor_mut()
        .and_then(|ed| resolve_save_as_path(&ed.path, input));
    state.dialogs.clear();
    if let Some(target) = target {
        save_active_editor(state, Some(target), false);
    }
}

impl Dialog for EditorSearchPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        EditorSearchPrompt::focused_field_mut(self)
    }
}

impl Dialog for EditorSaveAsPrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.input)
    }
}

impl Dialog for EditorConfirmOverwrite {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
}
