use crate::app::context::AppContext;
use crate::app::list_nav::handle_arrow_nav;
use crate::app::state::types::TreeViewCaller;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::TreeView {
        nodes, cursor_idx, ..
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    if handle_arrow_nav(key.code, cursor_idx, nodes.len()) {
        return Ok(None);
    }
    match key.code {
        KeyCode::Esc | KeyCode::F(10) => {
            if let Some(caller) = take_caller(state) {
                close(state, caller);
            }
        }
        KeyCode::Enter => {
            let Some(PopupType::TreeView {
                nodes, cursor_idx, ..
            }) = state.dialogs.top()
            else {
                return Err(());
            };
            let Some(target) = nodes.get(*cursor_idx).map(|node| {
                if node.is_dir {
                    node.path.clone()
                } else {
                    node.path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| node.path.clone())
                }
            }) else {
                return Ok(None);
            };
            if let Some(caller) = take_caller(state) {
                choose(state, context, caller, target);
            }
        }
        // Up / Down on an empty tree are swallowed.
        KeyCode::Up | KeyCode::Down => {}
        _ => return Err(()),
    }
    Ok(None)
}

/// Pops the tree view, returning who opened it.
fn take_caller(state: &mut AppState) -> Option<TreeViewCaller> {
    match state.dialogs.pop() {
        Some(PopupType::TreeView { caller, .. }) => Some(caller),
        Some(other) => {
            state.dialogs.push(other);
            None
        }
        None => None,
    }
}

/// Esc: back to where the tree was opened from.
fn close(state: &mut AppState, caller: TreeViewCaller) {
    match caller {
        TreeViewCaller::Panel(_) => state.dialogs.clear(),
        TreeViewCaller::TransferPrompt { previous } => state.dialogs.replace(*previous),
    }
}

/// Enter: jump the panel there, or fill the transfer destination.
fn choose(
    state: &mut AppState,
    context: &AppContext,
    caller: TreeViewCaller,
    target: std::path::PathBuf,
) {
    match caller {
        TreeViewCaller::Panel(panel) => {
            state.panels.side_mut(panel).open_path(target);
            state.dialogs.clear();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        TreeViewCaller::TransferPrompt { mut previous } => {
            if let PopupType::TransferPrompt(ref mut prompt) = *previous {
                prompt.input.set_text(target.to_string_lossy());
            }
            state.dialogs.replace(*previous);
        }
    }
}
