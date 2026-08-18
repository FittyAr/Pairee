use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::TreeView {
        nodes,
        cursor_idx,
        caller,
    }) = state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => {
                match caller {
                    crate::app::state::types::TreeViewCaller::Panel(_) => {
                        state.dialogs.clear();
                    }
                    crate::app::state::types::TreeViewCaller::CopyPrompt { previous } => {
                        state.dialogs.replace(*previous);
                    }
                    crate::app::state::types::TreeViewCaller::MovePrompt { previous } => {
                        state.dialogs.replace(*previous);
                    }
                }
                return Ok(None);
            }
            KeyCode::Up => {
                if !nodes.is_empty() {
                    let new_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        nodes.len() - 1
                    };
                    state.dialogs.replace(PopupType::TreeView {
                        nodes,
                        cursor_idx: new_idx,
                        caller,
                    });
                }
                return Ok(None);
            }
            KeyCode::Down => {
                if !nodes.is_empty() {
                    let new_idx = if cursor_idx < nodes.len() - 1 {
                        cursor_idx + 1
                    } else {
                        0
                    };
                    state.dialogs.replace(PopupType::TreeView {
                        nodes,
                        cursor_idx: new_idx,
                        caller,
                    });
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                if let Some(node) = nodes.get(cursor_idx) {
                    let target = if node.is_dir {
                        node.path.clone()
                    } else {
                        node.path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| node.path.clone())
                    };
                    match caller {
                        crate::app::state::types::TreeViewCaller::Panel(panel) => {
                            match panel {
                                crate::app::state::ActivePanel::Left => {
                                    state.panels.left.current_path = target;
                                    state.panels.left.cursor_index = 0;
                                    state.panels.left.clear_selection();
                                }
                                crate::app::state::ActivePanel::Right => {
                                    state.panels.right.current_path = target;
                                    state.panels.right.cursor_index = 0;
                                    state.panels.right.clear_selection();
                                }
                            }
                            state.dialogs.clear();
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        crate::app::state::types::TreeViewCaller::CopyPrompt { mut previous } => {
                            if let PopupType::CopyPrompt { ref mut input, .. } = *previous {
                                *input = target.to_string_lossy().to_string();
                            }
                            state.dialogs.replace(*previous);
                        }
                        crate::app::state::types::TreeViewCaller::MovePrompt { mut previous } => {
                            if let PopupType::MovePrompt { ref mut input, .. } = *previous {
                                *input = target.to_string_lossy().to_string();
                            }
                            state.dialogs.replace(*previous);
                        }
                    }
                }
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
