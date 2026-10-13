use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::popup::TreeView;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::state::types::TreeViewCaller;
use crate::app::state::{AppState, CopyMovePromptState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::TreeView(TreeView {
        nodes, cursor_idx, ..
    })) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match list_key(ListKeys::ARROWS, key.code, cursor_idx, nodes.len()) {
        ListKey::Moved => {}
        ListKey::Close | ListKey::Other if matches!(key.code, KeyCode::Esc | KeyCode::F(10)) => {
            if let Some(caller) = take_caller(state) {
                close(state, caller);
            }
        }
        ListKey::Activate(_) => {
            let Some(PopupType::TreeView(TreeView {
                nodes, cursor_idx, ..
            })) = state.dialogs.top()
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
        ListKey::Close | ListKey::Other => return Err(()),
    }
    Ok(None)
}

/// Pops the tree view, returning who opened it.
fn take_caller(state: &mut AppState) -> Option<TreeViewCaller> {
    state.dialogs.pop_as::<TreeView>().map(|tree| tree.caller)
}

/// Esc: back to where the tree was opened from.
fn close(state: &mut AppState, caller: TreeViewCaller) {
    match caller {
        TreeViewCaller::Panel(_) => state.dialogs.clear(),
        // The Copy / Move dialog underneath is on top again.
        TreeViewCaller::TransferPrompt => {}
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
        TreeViewCaller::TransferPrompt => {
            if let Some(prompt) = state.dialogs.top_mut_as::<CopyMovePromptState>() {
                prompt.input.set_text(target.to_string_lossy());
            }
        }
    }
}

impl Dialog for TreeView {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn closes_on_outside_click(&self) -> bool {
        true
    }
    fn resizable(&self) -> bool {
        true
    }
}
