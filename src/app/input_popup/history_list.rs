use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        let is_alt = key.modifiers.contains(KeyModifiers::ALT);

        match p {
            PopupType::CommandHistoryList {
                mut entries,
                mut cursor_idx,
            } => {
                match key.code {
                    KeyCode::Up => {
                        if !entries.is_empty() {
                            if cursor_idx > 0 {
                                cursor_idx -= 1;
                            } else {
                                cursor_idx = entries.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !entries.is_empty() {
                            if cursor_idx < entries.len() - 1 {
                                cursor_idx += 1;
                            } else {
                                cursor_idx = 0;
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        cursor_idx = cursor_idx.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        if !entries.is_empty() {
                            cursor_idx = (cursor_idx + 10).min(entries.len() - 1);
                        }
                    }
                    KeyCode::Home => {
                        cursor_idx = 0;
                    }
                    KeyCode::End => {
                        if !entries.is_empty() {
                            cursor_idx = entries.len() - 1;
                        }
                    }
                    KeyCode::Enter => {
                        if !entries.is_empty() && cursor_idx < entries.len() {
                            state.cli_input = entries[cursor_idx].clone();
                        }
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Delete if is_alt => {
                        // Alt+Delete: Clear entire history list
                        if context.config.settings.confirmations.confirm_clear_history_list {
                            state.active_popup = Some(PopupType::ConfirmClearHistory {
                                history_type: "command".to_string(),
                            });
                        } else {
                            state.command_history.clear();
                            let mut history_store = crate::config::history::HistoryStore::default();
                            history_store.commands = state.command_history.clone();
                            history_store.viewed_files = state.file_view_history.clone();
                            history_store.visited_folders = state.folders_history.clone();
                            let _ = history_store.save();
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Delete => {
                        // Delete single item from history list
                        if !entries.is_empty() && cursor_idx < entries.len() {
                            entries.remove(cursor_idx);
                            state.command_history = entries.clone();
                            let mut history_store = crate::config::history::HistoryStore::default();
                            history_store.commands = state.command_history.clone();
                            history_store.viewed_files = state.file_view_history.clone();
                            history_store.visited_folders = state.folders_history.clone();
                            let _ = history_store.save();

                            if entries.is_empty() {
                                state.active_popup = None;
                                return Ok(None);
                            } else if cursor_idx >= entries.len() {
                                cursor_idx = entries.len() - 1;
                            }
                        }
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::CommandHistoryList {
                    entries,
                    cursor_idx,
                });
                return Ok(None);
            }
            PopupType::FileViewHistoryList {
                mut entries,
                mut cursor_idx,
            } => {
                match key.code {
                    KeyCode::Up => {
                        if !entries.is_empty() {
                            if cursor_idx > 0 {
                                cursor_idx -= 1;
                            } else {
                                cursor_idx = entries.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !entries.is_empty() {
                            if cursor_idx < entries.len() - 1 {
                                cursor_idx += 1;
                            } else {
                                cursor_idx = 0;
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        cursor_idx = cursor_idx.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        if !entries.is_empty() {
                            cursor_idx = (cursor_idx + 10).min(entries.len() - 1);
                        }
                    }
                    KeyCode::Home => {
                        cursor_idx = 0;
                    }
                    KeyCode::End => {
                        if !entries.is_empty() {
                            cursor_idx = entries.len() - 1;
                        }
                    }
                    KeyCode::Enter => {
                        if !entries.is_empty() && cursor_idx < entries.len() {
                            let path = entries[cursor_idx].clone();
                            state.active_popup = None;
                            let viewer = crate::ui::viewer::ViewerState::load(path);
                            state.active_popup = Some(PopupType::InternalViewer { viewer });
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Delete if is_alt => {
                        // Alt+Delete: Clear entire history list
                        if context.config.settings.confirmations.confirm_clear_history_list {
                            state.active_popup = Some(PopupType::ConfirmClearHistory {
                                history_type: "view".to_string(),
                            });
                        } else {
                            state.file_view_history.clear();
                            let mut history_store = crate::config::history::HistoryStore::default();
                            history_store.commands = state.command_history.clone();
                            history_store.viewed_files = state.file_view_history.clone();
                            history_store.visited_folders = state.folders_history.clone();
                            let _ = history_store.save();
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Delete => {
                        // Delete single item from history list
                        if !entries.is_empty() && cursor_idx < entries.len() {
                            entries.remove(cursor_idx);
                            state.file_view_history = entries.clone();
                            let mut history_store = crate::config::history::HistoryStore::default();
                            history_store.commands = state.command_history.clone();
                            history_store.viewed_files = state.file_view_history.clone();
                            history_store.visited_folders = state.folders_history.clone();
                            let _ = history_store.save();

                            if entries.is_empty() {
                                state.active_popup = None;
                                return Ok(None);
                            } else if cursor_idx >= entries.len() {
                                cursor_idx = entries.len() - 1;
                            }
                        }
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::FileViewHistoryList {
                    entries,
                    cursor_idx,
                });
                return Ok(None);
            }
            PopupType::FoldersHistoryList {
                mut entries,
                mut cursor_idx,
            } => {
                match key.code {
                    KeyCode::Up => {
                        if !entries.is_empty() {
                            if cursor_idx > 0 {
                                cursor_idx -= 1;
                            } else {
                                cursor_idx = entries.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !entries.is_empty() {
                            if cursor_idx < entries.len() - 1 {
                                cursor_idx += 1;
                            } else {
                                cursor_idx = 0;
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        cursor_idx = cursor_idx.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        if !entries.is_empty() {
                            cursor_idx = (cursor_idx + 10).min(entries.len() - 1);
                        }
                    }
                    KeyCode::Home => {
                        cursor_idx = 0;
                    }
                    KeyCode::End => {
                        if !entries.is_empty() {
                            cursor_idx = entries.len() - 1;
                        }
                    }
                    KeyCode::Enter => {
                        if !entries.is_empty() && cursor_idx < entries.len() {
                            let path = entries[cursor_idx].clone();
                            let panel = state.get_active_panel_mut();
                            panel.current_path = path;
                            panel.cursor_index = 0;
                            panel.selected_paths.clear();
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Delete if is_alt => {
                        // Alt+Delete: Clear entire history list
                        if context.config.settings.confirmations.confirm_clear_history_list {
                            state.active_popup = Some(PopupType::ConfirmClearHistory {
                                history_type: "folder".to_string(),
                            });
                        } else {
                            state.folders_history.clear();
                            let mut history_store = crate::config::history::HistoryStore::default();
                            history_store.commands = state.command_history.clone();
                            history_store.viewed_files = state.file_view_history.clone();
                            history_store.visited_folders = state.folders_history.clone();
                            let _ = history_store.save();
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Delete => {
                        // Delete single item from history list
                        if !entries.is_empty() && cursor_idx < entries.len() {
                            entries.remove(cursor_idx);
                            state.folders_history = entries.clone();
                            let mut history_store = crate::config::history::HistoryStore::default();
                            history_store.commands = state.command_history.clone();
                            history_store.viewed_files = state.file_view_history.clone();
                            history_store.visited_folders = state.folders_history.clone();
                            let _ = history_store.save();

                            if entries.is_empty() {
                                state.active_popup = None;
                                return Ok(None);
                            } else if cursor_idx >= entries.len() {
                                cursor_idx = entries.len() - 1;
                            }
                        }
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::FoldersHistoryList {
                    entries,
                    cursor_idx,
                });
                return Ok(None);
            }
            _ => Err(()),
        }
    } else {
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn test_history_list_navigation() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let config = AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        };
        let mut context = AppContext::new(config);

        let entries = vec!["cmd1".to_string(), "cmd2".to_string(), "cmd3".to_string()];
        state.active_popup = Some(PopupType::CommandHistoryList {
            entries: entries.clone(),
            cursor_idx: 0,
        });

        // Test Down key
        let res = handle(&mut state, make_key(KeyCode::Down, KeyModifiers::empty()), &mut context);
        assert!(res.is_ok());
        if let Some(PopupType::CommandHistoryList { cursor_idx, .. }) = state.active_popup {
            assert_eq!(cursor_idx, 1);
        } else {
            panic!("Expected CommandHistoryList popup");
        }

        // Test Up key
        let res = handle(&mut state, make_key(KeyCode::Up, KeyModifiers::empty()), &mut context);
        assert!(res.is_ok());
        if let Some(PopupType::CommandHistoryList { cursor_idx, .. }) = state.active_popup {
            assert_eq!(cursor_idx, 0);
        } else {
            panic!("Expected CommandHistoryList popup");
        }
    }
}
