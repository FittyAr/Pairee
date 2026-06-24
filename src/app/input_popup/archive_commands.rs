use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType, Screen};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::Path;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::ArchiveCommandsMenu {
        archive_path,
        items,
        mut cursor_idx,
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if !items.is_empty() {
                    cursor_idx = if cursor_idx > 0 {
                        cursor_idx - 1
                    } else {
                        items.len() - 1
                    };
                    state.active_popup = Some(PopupType::ArchiveCommandsMenu {
                        archive_path,
                        items,
                        cursor_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !items.is_empty() {
                    cursor_idx = if cursor_idx < items.len() - 1 {
                        cursor_idx + 1
                    } else {
                        0
                    };
                    state.active_popup = Some(PopupType::ArchiveCommandsMenu {
                        archive_path,
                        items,
                        cursor_idx,
                    });
                }
                return Ok(None);
            }
            KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') | KeyCode::Char('4') => {
                let chosen_idx = match key.code {
                    KeyCode::Char('1') => 0,
                    KeyCode::Char('2') => 1,
                    KeyCode::Char('3') => 2,
                    KeyCode::Char('4') => 3,
                    _ => 0,
                };
                if chosen_idx < items.len() {
                    execute_option(state, &archive_path, chosen_idx);
                }
                return Ok(None);
            }
            KeyCode::Enter => {
                execute_option(state, &archive_path, cursor_idx);
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}

fn execute_option(state: &mut AppState, archive_path: &Path, cursor_idx: usize) {
    state.active_popup = None;
    match cursor_idx {
        0 => {
            // List contents
            match crate::fs::archive::list_archive_files(archive_path) {
                Ok(list) => {
                    let viewer = crate::ui::viewer::ViewerState {
                        path: archive_path.to_path_buf(),
                        lines: list,
                        raw: Vec::new(),
                        image_data: None,
                        is_image: false,
                        is_text: true,
                        mode: crate::ui::viewer::ViewerMode::Text,
                        scroll: 0,
                        last_search: None,
                        last_case_sensitive: false,
                    };
                    state.push_screen(Screen::Viewer(viewer));
                }
                Err(e) => {
                    state.active_popup =
                        Some(PopupType::Error(format!("Failed to list archive: {}", e)));
                }
            }
        }
        1 => {
            // Test integrity
            match crate::fs::archive::list_archive_files(archive_path) {
                Ok(_) => {
                    state.active_popup = Some(PopupType::Info(crate::config::localization::t(
                        "archive_test_ok",
                    )));
                }
                Err(e) => {
                    state.active_popup = Some(PopupType::Error(format!(
                        "Archive integrity check failed: {}",
                        e
                    )));
                }
            }
        }
        2 | 3 => {
            // Extract
            let dest = if cursor_idx == 2 {
                state.get_active_panel().current_path.clone()
            } else {
                state.get_passive_panel().current_path.clone()
            };
            let rx = crate::fs::spawn_extract_task(archive_path.to_path_buf(), dest);
            state.progress_rx = Some(rx);
            state.active_popup = Some(PopupType::CopyProgress {
                is_move: false,
                current_file: crate::config::localization::t("progress_extracting"),
                files_copied: 0,
                total_files: 0,
                bytes_copied: 0,
                total_bytes: 0,
            });
        }
        _ => {}
    }
}
