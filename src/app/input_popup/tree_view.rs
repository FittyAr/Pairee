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
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => {
                match caller {
                    crate::app::state::types::TreeViewCaller::Panel(_) => {
                        state.active_popup = None;
                    }
                    crate::app::state::types::TreeViewCaller::CopyPrompt { previous } => {
                        state.active_popup = Some(*previous);
                    }
                    crate::app::state::types::TreeViewCaller::RenMovPrompt { previous } => {
                        state.active_popup = Some(*previous);
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
                    state.active_popup = Some(PopupType::TreeView {
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
                    state.active_popup = Some(PopupType::TreeView {
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
                                    state.left_panel.current_path = target;
                                    state.left_panel.cursor_index = 0;
                                    state.left_panel.clear_selection();
                                }
                                crate::app::state::ActivePanel::Right => {
                                    state.right_panel.current_path = target;
                                    state.right_panel.cursor_index = 0;
                                    state.right_panel.clear_selection();
                                }
                            }
                            state.active_popup = None;
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        crate::app::state::types::TreeViewCaller::CopyPrompt { mut previous } => {
                            if let PopupType::CopyPrompt { ref mut input, .. } = *previous {
                                *input = target.to_string_lossy().to_string();
                            }
                            state.active_popup = Some(*previous);
                        }
                        crate::app::state::types::TreeViewCaller::RenMovPrompt { mut previous } => {
                            if let PopupType::RenMovPrompt { ref mut input, .. } = *previous {
                                *input = target.to_string_lossy().to_string();
                            }
                            state.active_popup = Some(*previous);
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
