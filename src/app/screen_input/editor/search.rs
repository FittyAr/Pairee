//! Editor search commands: opening the find / replace dialog and repeating
//! the last search (F3 / Shift+F7 forward, Alt+F7 / Shift+F3 backward).

use crate::app::editor::search::FindOutcome;
use crate::app::state::popup::EditorSearchPrompt;
use crate::app::state::{AppState, PopupType, Screen};
use crate::app::text_input::TextField;
use crate::config::localization::t;

/// Opens the find dialog (`replace`: the replace dialog) filled with the
/// selected text (one line) or the last search, else the newest history
/// entries.
pub(super) fn open_dialog(state: &mut AppState, replace: bool) {
    let history = &state.history;
    let Screen::Editor(ed) = state.screens.active() else {
        return;
    };
    let last = ed.last_search.as_ref();
    let selected = ed.selected_text().filter(|s| !s.contains('\n'));
    let query = selected
        .or_else(|| last.map(|s| s.pattern.clone()))
        .or_else(|| history.searches.first().cloned())
        .unwrap_or_default();
    let replacement = replace.then(|| {
        let text = last
            .and_then(|s| s.replacement.clone())
            .or_else(|| history.replacements.first().cloned())
            .unwrap_or_default();
        TextField::new(text)
    });
    let prompt = EditorSearchPrompt {
        query: TextField::new(query),
        replacement,
        options: last.map(|s| s.options).unwrap_or_default(),
        cursor_idx: 0,
    };
    state
        .dialogs
        .replace(PopupType::EditorSearchPrompt(Box::new(prompt)));
}

/// Repeats the last search (`reverse`: the other way); opens the find
/// dialog when there is none yet.
pub(super) fn repeat(state: &mut AppState, reverse: bool, height: usize) {
    let outcome = state
        .active_editor_mut()
        .and_then(|ed| ed.repeat_search(reverse, height));
    match outcome {
        None => open_dialog(state, false),
        Some(outcome) => report(state, outcome),
    }
}

/// Tells the user when a search found nothing.
pub fn report(state: &mut AppState, outcome: FindOutcome) {
    let key = match outcome {
        FindOutcome::Found => return,
        FindOutcome::NotFound => "editor_text_not_found",
        FindOutcome::Locked => "editor_read_only_locked",
    };
    state.dialogs.push(PopupType::error(t(key)));
}
