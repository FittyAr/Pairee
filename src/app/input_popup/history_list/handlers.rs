//! Handlers for Command, FileView, and Folder history lists.

use super::nav::handle_list_nav;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_command_history(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
    mut entries: Vec<String>,
    mut cursor_idx: usize,
) -> Result<Option<Action>, ()> {
    let is_alt = key.modifiers.contains(KeyModifiers::ALT);
    if handle_list_nav(key.code, &mut cursor_idx, entries.len()) {
        state.dialogs.replace(PopupType::CommandHistoryList {
            entries,
            cursor_idx,
        });
        return Ok(None);
    }
    match key.code {
        KeyCode::Enter => {
            if !entries.is_empty() && cursor_idx < entries.len() {
                state.cli_input = entries[cursor_idx].clone();
            }
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Esc => {
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Delete if is_alt => {
            if context
                .config
                .settings
                .confirmations
                .confirm_clear_history_list
            {
                state.dialogs.replace(PopupType::ConfirmClearHistory {
                    history_type: "command".to_string(),
                });
            } else {
                state.history.commands.clear();
                let history_store = crate::config::history::HistoryStore {
                    commands: state.history.commands.clone(),
                    viewed_files: state.history.viewed_files.clone(),
                    visited_folders: state.history.folders.clone(),
                };
                let _ = history_store.save();
                state.dialogs.clear();
            }
            Ok(None)
        }
        KeyCode::Delete if !entries.is_empty() && cursor_idx < entries.len() => {
            entries.remove(cursor_idx);
            state.history.commands = entries.clone();
            let history_store = crate::config::history::HistoryStore {
                commands: state.history.commands.clone(),
                viewed_files: state.history.viewed_files.clone(),
                visited_folders: state.history.folders.clone(),
            };
            let _ = history_store.save();

            if entries.is_empty() {
                state.dialogs.clear();
            } else {
                if cursor_idx >= entries.len() {
                    cursor_idx = entries.len() - 1;
                }
                state.dialogs.replace(PopupType::CommandHistoryList {
                    entries,
                    cursor_idx,
                });
            }
            Ok(None)
        }
        _ => {
            state.dialogs.replace(PopupType::CommandHistoryList {
                entries,
                cursor_idx,
            });
            Ok(None)
        }
    }
}

pub fn handle_file_view_history(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
    mut entries: Vec<std::path::PathBuf>,
    mut cursor_idx: usize,
) -> Result<Option<Action>, ()> {
    let is_alt = key.modifiers.contains(KeyModifiers::ALT);
    if handle_list_nav(key.code, &mut cursor_idx, entries.len()) {
        state.dialogs.replace(PopupType::FileViewHistoryList {
            entries,
            cursor_idx,
        });
        return Ok(None);
    }
    match key.code {
        KeyCode::Enter => {
            if !entries.is_empty() && cursor_idx < entries.len() {
                let path = entries[cursor_idx].clone();
                state.dialogs.clear();
                let viewer = crate::ui::viewer::ViewerState::load_with_images(
                    path,
                    context.config.settings.image_preview_enabled,
                );
                state.push_screen(crate::app::state::Screen::Viewer(viewer));
            } else {
                state.dialogs.clear();
            }
            Ok(None)
        }
        KeyCode::Esc => {
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Delete if is_alt => {
            if context
                .config
                .settings
                .confirmations
                .confirm_clear_history_list
            {
                state.dialogs.replace(PopupType::ConfirmClearHistory {
                    history_type: "view".to_string(),
                });
            } else {
                state.history.viewed_files.clear();
                let history_store = crate::config::history::HistoryStore {
                    commands: state.history.commands.clone(),
                    viewed_files: state.history.viewed_files.clone(),
                    visited_folders: state.history.folders.clone(),
                };
                let _ = history_store.save();
                state.dialogs.clear();
            }
            Ok(None)
        }
        KeyCode::Delete if !entries.is_empty() && cursor_idx < entries.len() => {
            entries.remove(cursor_idx);
            state.history.viewed_files = entries.clone();
            let history_store = crate::config::history::HistoryStore {
                commands: state.history.commands.clone(),
                viewed_files: state.history.viewed_files.clone(),
                visited_folders: state.history.folders.clone(),
            };
            let _ = history_store.save();

            if entries.is_empty() {
                state.dialogs.clear();
            } else {
                if cursor_idx >= entries.len() {
                    cursor_idx = entries.len() - 1;
                }
                state.dialogs.replace(PopupType::FileViewHistoryList {
                    entries,
                    cursor_idx,
                });
            }
            Ok(None)
        }
        _ => {
            state.dialogs.replace(PopupType::FileViewHistoryList {
                entries,
                cursor_idx,
            });
            Ok(None)
        }
    }
}

pub fn handle_folders_history(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
    mut entries: Vec<std::path::PathBuf>,
    mut cursor_idx: usize,
) -> Result<Option<Action>, ()> {
    let is_alt = key.modifiers.contains(KeyModifiers::ALT);
    if handle_list_nav(key.code, &mut cursor_idx, entries.len()) {
        state.dialogs.replace(PopupType::FoldersHistoryList {
            entries,
            cursor_idx,
        });
        return Ok(None);
    }
    match key.code {
        KeyCode::Enter => {
            if !entries.is_empty() && cursor_idx < entries.len() {
                let path = entries[cursor_idx].clone();
                let panel = state.get_active_panel_mut();
                panel.current_path = path;
                panel.cursor_index = 0;
                panel.clear_selection();
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Esc => {
            state.dialogs.clear();
            Ok(None)
        }
        KeyCode::Delete if is_alt => {
            if context
                .config
                .settings
                .confirmations
                .confirm_clear_history_list
            {
                state.dialogs.replace(PopupType::ConfirmClearHistory {
                    history_type: "folder".to_string(),
                });
            } else {
                state.history.folders.clear();
                let history_store = crate::config::history::HistoryStore {
                    commands: state.history.commands.clone(),
                    viewed_files: state.history.viewed_files.clone(),
                    visited_folders: state.history.folders.clone(),
                };
                let _ = history_store.save();
                state.dialogs.clear();
            }
            Ok(None)
        }
        KeyCode::Delete if !entries.is_empty() && cursor_idx < entries.len() => {
            entries.remove(cursor_idx);
            state.history.folders = entries.clone();
            let history_store = crate::config::history::HistoryStore {
                commands: state.history.commands.clone(),
                viewed_files: state.history.viewed_files.clone(),
                visited_folders: state.history.folders.clone(),
            };
            let _ = history_store.save();

            if entries.is_empty() {
                state.dialogs.clear();
            } else {
                if cursor_idx >= entries.len() {
                    cursor_idx = entries.len() - 1;
                }
                state.dialogs.replace(PopupType::FoldersHistoryList {
                    entries,
                    cursor_idx,
                });
            }
            Ok(None)
        }
        _ => {
            state.dialogs.replace(PopupType::FoldersHistoryList {
                entries,
                cursor_idx,
            });
            Ok(None)
        }
    }
}
