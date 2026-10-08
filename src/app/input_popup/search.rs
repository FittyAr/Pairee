//! Find file dialog and its results list.

use crate::app::context::AppContext;
use crate::app::form::{FormKey, FormLayout};
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

/// Name pattern, content, case, target, then [OK] [Cancel].
pub const SEARCH_FORM: FormLayout = FormLayout::new(6, 4);
const ROW_CASE: usize = 2;
const ROW_TARGET: usize = 3;
const BUTTON_CANCEL: usize = 5;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    match state.dialogs.top() {
        Some(PopupType::SearchPrompt { .. }) => handle_prompt(state, key),
        Some(PopupType::SearchResults { .. }) => handle_results(state, key, context),
        _ => Err(()),
    }
}

fn handle_prompt(state: &mut AppState, key: KeyEvent) -> Result<Option<Action>, ()> {
    let Some(PopupType::SearchPrompt {
        query,
        content_query,
        case_sensitive,
        search_target,
        cursor_idx,
        ..
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    if *cursor_idx == ROW_TARGET && matches!(key.code, KeyCode::Left | KeyCode::Right) {
        *search_target = search_target.cycle(key.code == KeyCode::Right);
        return Ok(None);
    }
    let field = match *cursor_idx {
        0 => Some(query),
        1 => Some(content_query),
        _ => None,
    };
    match SEARCH_FORM.handle(cursor_idx, field, &key) {
        FormKey::Toggle(ROW_CASE) => *case_sensitive = !*case_sensitive,
        FormKey::Toggle(ROW_TARGET) => *search_target = search_target.cycle(true),
        FormKey::Activate(BUTTON_CANCEL) | FormKey::Cancel => state.dialogs.clear(),
        FormKey::Activate(_) => start_search(state),
        FormKey::Toggle(_) | FormKey::Handled => {}
        FormKey::Other => return Err(()),
    }
    Ok(None)
}

/// Starts the background search and shows the (filling) results list.
fn start_search(state: &mut AppState) {
    let Some(PopupType::SearchPrompt {
        query,
        content_query,
        search_root,
        case_sensitive,
        search_target,
        ..
    }) = state.dialogs.top()
    else {
        return;
    };
    let (name, content) = (query.text().to_string(), content_query.text().to_string());
    if name.is_empty() && content.is_empty() {
        state.dialogs.clear();
        return;
    }
    let name_glob = if name.is_empty() || name.contains(['*', '?']) {
        name.clone()
    } else {
        format!("*{}*", name)
    };
    let search = crate::fs::search::SearchQuery {
        name_glob,
        content: (!content.is_empty()).then(|| content.clone()),
        root: search_root.clone(),
        case_sensitive: *case_sensitive,
        target: *search_target,
    };
    state.search_rx = Some(crate::fs::search::find_files(search));
    state.dialogs.replace(PopupType::SearchResults {
        query: if name.is_empty() { content } else { name },
        results: Vec::new(),
        cursor_idx: 0,
        searching: true,
    });
}

fn handle_results(
    state: &mut AppState,
    key: KeyEvent,
    context: &AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::SearchResults {
        results,
        cursor_idx,
        ..
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match list_key(ListKeys::ARROWS, key.code, cursor_idx, results.len()) {
        ListKey::Moved => {}
        ListKey::Close => {
            // Esc also cancels a search still running.
            state.search_rx = None;
            state.dialogs.clear();
        }
        ListKey::Activate(idx) => {
            let Some((path, is_dir)) = results.get(idx) else {
                return Ok(None);
            };
            let target = if *is_dir {
                path.clone()
            } else {
                path.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| path.clone())
            };
            state.search_rx = None;
            state.get_active_panel_mut().open_path(target);
            state.dialogs.clear();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        ListKey::Other => return Err(()),
    }
    Ok(None)
}
