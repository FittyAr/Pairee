use super::menu_handler::trigger_menu_item;
use super::sys_helpers::{
    change_preset, change_theme, find_next_in_editor, kill_process, search_files_recursive,
};
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Captures keyboard input for active popups.
pub fn handle_popup_input(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::MkDirPrompt { ref input } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::MkDirPrompt { input: new_input });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::MkDirPrompt { input: new_input });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if !input.is_empty() {
                            let path = state.get_active_panel().current_path.join(input);
                            if let Err(e) = crate::fs::create_directory(&path) {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Directory error: {}", e)));
                            } else {
                                state.active_popup = None;
                                state.refresh_both_panels(context.config.settings.show_hidden);
                            }
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::ConfirmDelete { ref paths } => {
                match key.code {
                    KeyCode::Enter => {
                        for path in paths {
                            if let Err(e) = crate::fs::delete_sync(path) {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Delete failed: {}", e)));
                                return Ok(None);
                            }
                        }
                        state.active_popup = None;
                        state.get_active_panel_mut().selected_paths.clear();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::Error(_) | PopupType::Help | PopupType::Info(_) => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    state.active_popup = None;
                    return Ok(None);
                }
                return Err(());
            }
            PopupType::CopyProgress { .. } => {
                if key.code == KeyCode::Esc {
                    // Drop channel to signal abort to tokio background thread
                    state.progress_rx = None;
                    state.active_popup = None;
                    state.refresh_both_panels(context.config.settings.show_hidden);
                    return Ok(None);
                }
                return Err(());
            }
            PopupType::UserMenu => {
                match key.code {
                    KeyCode::Char('1') => {
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Char('2') => {
                        context.config.settings.show_hidden = !context.config.settings.show_hidden;
                        let _ = context.config.save();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Char('3') => {
                        state.swap_panels();
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Char('4') => {
                        state.active_popup = Some(PopupType::Help);
                        return Ok(None);
                    }
                    KeyCode::Char('5') | KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Char('6') => {
                        state.active_popup = None;
                        let (tx, rx) = tokio::sync::mpsc::channel(100);
                        tokio::spawn(async move {
                            let _ = tx
                                .send(crate::fs::ProgressUpdate {
                                    current_file: "Downloading 7z...".to_string(),
                                    files_copied: 0,
                                    total_files: 1,
                                    bytes_copied: 0,
                                    total_bytes: 1,
                                    error: None,
                                })
                                .await;

                            if let Err(e) = crate::fs::external_tools::ensure_external_tools().await
                            {
                                let _ = tx
                                    .send(crate::fs::ProgressUpdate {
                                        current_file: "Completed".to_string(),
                                        files_copied: 0,
                                        total_files: 1,
                                        bytes_copied: 0,
                                        total_bytes: 1,
                                        error: Some(format!("Failed to download: {}", e)),
                                    })
                                    .await;
                            } else {
                                let _ = tx
                                    .send(crate::fs::ProgressUpdate {
                                        current_file: "Completed".to_string(),
                                        files_copied: 1,
                                        total_files: 1,
                                        bytes_copied: 1,
                                        total_bytes: 1,
                                        error: None,
                                    })
                                    .await;
                            }
                        });

                        state.progress_rx = Some(rx);
                        state.active_popup = Some(PopupType::CopyProgress {
                            current_file: "Initializing Download...".to_string(),
                            files_copied: 0,
                            total_files: 1,
                            bytes_copied: 0,
                            total_bytes: 1,
                        });
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::InternalEditor {
                path,
                mut lines,
                mut cursor_x,
                mut cursor_y,
                mut scroll_y,
                mut is_dirty,
                last_search,
            } => {
                let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
                let edit_height = ((term_height as u16 * 90 / 100).saturating_sub(3)) as usize;

                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                match key.code {
                    KeyCode::Char(c) if !is_ctrl => {
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        let line = &mut lines[cursor_y];
                        if cursor_x <= line.len() {
                            line.insert(cursor_x, c);
                            cursor_x += 1;
                            is_dirty = true;
                        }
                    }
                    KeyCode::Backspace => {
                        if cursor_x > 0 {
                            let line = &mut lines[cursor_y];
                            line.remove(cursor_x - 1);
                            cursor_x -= 1;
                            is_dirty = true;
                        } else if cursor_y > 0 {
                            let current_line = lines.remove(cursor_y);
                            cursor_y -= 1;
                            let prev_line_len = lines[cursor_y].len();
                            lines[cursor_y].push_str(&current_line);
                            cursor_x = prev_line_len;
                            is_dirty = true;
                        }
                    }
                    KeyCode::Delete => {
                        if cursor_y < lines.len() {
                            let line = &mut lines[cursor_y];
                            if cursor_x < line.len() {
                                line.remove(cursor_x);
                                is_dirty = true;
                            } else if cursor_y < lines.len() - 1 {
                                let next_line = lines.remove(cursor_y + 1);
                                lines[cursor_y].push_str(&next_line);
                                is_dirty = true;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        let current_line = &mut lines[cursor_y];
                        let next_line = current_line.split_off(cursor_x);
                        lines.insert(cursor_y + 1, next_line);
                        cursor_y += 1;
                        cursor_x = 0;
                        is_dirty = true;
                    }
                    KeyCode::Up => {
                        if cursor_y > 0 {
                            cursor_y -= 1;
                            cursor_x = cursor_x.min(lines[cursor_y].len());
                            if cursor_y < scroll_y {
                                scroll_y = cursor_y;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if cursor_y < lines.len().saturating_sub(1) {
                            cursor_y += 1;
                            cursor_x = cursor_x.min(lines[cursor_y].len());
                            if cursor_y >= scroll_y + edit_height {
                                scroll_y = cursor_y.saturating_sub(edit_height - 1);
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        cursor_y = cursor_y.saturating_sub(edit_height);
                        cursor_x = cursor_x.min(lines[cursor_y].len());
                        if cursor_y < scroll_y {
                            scroll_y = cursor_y;
                        }
                    }
                    KeyCode::PageDown => {
                        cursor_y = (cursor_y + edit_height).min(lines.len().saturating_sub(1));
                        cursor_x = cursor_x.min(lines[cursor_y].len());
                        if cursor_y >= scroll_y + edit_height {
                            scroll_y = cursor_y.saturating_sub(edit_height - 1);
                        }
                    }
                    KeyCode::Left => {
                        if cursor_x > 0 {
                            cursor_x -= 1;
                        } else if cursor_y > 0 {
                            cursor_y -= 1;
                            cursor_x = lines[cursor_y].len();
                        }
                    }
                    KeyCode::Right => {
                        if cursor_y < lines.len() {
                            let line_len = lines[cursor_y].len();
                            if cursor_x < line_len {
                                cursor_x += 1;
                            } else if cursor_y < lines.len() - 1 {
                                cursor_y += 1;
                                cursor_x = 0;
                            }
                        }
                    }
                    KeyCode::F(2) => {
                        let content = lines.join("\n");
                        if let Err(e) = std::fs::write(&path, content) {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to save: {}", e)));
                            return Ok(None);
                        }
                        is_dirty = false;
                    }
                    KeyCode::Char('s') if is_ctrl => {
                        let content = lines.join("\n");
                        if let Err(e) = std::fs::write(&path, content) {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to save: {}", e)));
                            return Ok(None);
                        }
                        is_dirty = false;
                    }
                    KeyCode::Char('r') if is_ctrl => match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            let reloaded_lines: Vec<String> =
                                content.lines().map(|s| s.to_string()).collect();
                            lines = if reloaded_lines.is_empty() {
                                vec![String::new()]
                            } else {
                                reloaded_lines
                            };
                            cursor_x =
                                cursor_x.min(lines.get(cursor_y).map(|l| l.len()).unwrap_or(0));
                            is_dirty = false;
                        }
                        Err(e) => {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to reload: {}", e)));
                            return Ok(None);
                        }
                    },
                    KeyCode::Char('d') if is_ctrl => match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            let reloaded_lines: Vec<String> =
                                content.lines().map(|s| s.to_string()).collect();
                            lines = if reloaded_lines.is_empty() {
                                vec![String::new()]
                            } else {
                                reloaded_lines
                            };
                            cursor_x =
                                cursor_x.min(lines.get(cursor_y).map(|l| l.len()).unwrap_or(0));
                            is_dirty = false;
                        }
                        Err(e) => {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to reload: {}", e)));
                            return Ok(None);
                        }
                    },
                    KeyCode::F(7) if is_shift => {
                        if let Some(ref q) = last_search {
                            if let Some((found_x, found_y)) =
                                find_next_in_editor(&lines, cursor_x, cursor_y, q)
                            {
                                cursor_x = found_x;
                                cursor_y = found_y;
                                if cursor_y < scroll_y || cursor_y >= scroll_y + edit_height {
                                    scroll_y = cursor_y.saturating_sub(edit_height / 2);
                                }
                            }
                        }
                    }
                    KeyCode::F(7) => {
                        state.active_popup = Some(PopupType::EditorSearchPrompt {
                            path,
                            lines,
                            cursor_x,
                            cursor_y,
                            scroll_y,
                            is_dirty,
                            last_search,
                            query: String::new(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Char('f') if is_ctrl => {
                        state.active_popup = Some(PopupType::EditorSearchPrompt {
                            path,
                            lines,
                            cursor_x,
                            cursor_y,
                            scroll_y,
                            is_dirty,
                            last_search,
                            query: String::new(),
                        });
                        return Ok(None);
                    }
                    KeyCode::F(3) => {
                        if let Some(ref q) = last_search {
                            if let Some((found_x, found_y)) =
                                find_next_in_editor(&lines, cursor_x, cursor_y, q)
                            {
                                cursor_x = found_x;
                                cursor_y = found_y;
                                if cursor_y < scroll_y || cursor_y >= scroll_y + edit_height {
                                    scroll_y = cursor_y.saturating_sub(edit_height / 2);
                                }
                            }
                        }
                    }
                    KeyCode::Esc | KeyCode::F(10) => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::InternalEditor {
                    path,
                    lines,
                    cursor_x,
                    cursor_y,
                    scroll_y,
                    is_dirty,
                    last_search,
                });
                return Ok(None);
            }
            PopupType::EditorSearchPrompt {
                path,
                lines,
                cursor_x,
                cursor_y,
                scroll_y,
                is_dirty,
                last_search,
                mut query,
            } => {
                let term_height = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
                let edit_height = ((term_height as u16 * 90 / 100).saturating_sub(3)) as usize;

                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Char(c) if !is_ctrl => {
                        query.push(c);
                    }
                    KeyCode::Backspace => {
                        query.pop();
                    }
                    KeyCode::Esc => {
                        state.active_popup = Some(PopupType::InternalEditor {
                            path,
                            lines,
                            cursor_x,
                            cursor_y,
                            scroll_y,
                            is_dirty,
                            last_search,
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let q = query.clone();
                        if !q.is_empty() {
                            if let Some((found_x, found_y)) =
                                find_next_in_editor(&lines, cursor_x, cursor_y, &q)
                            {
                                let new_cursor_x = found_x;
                                let new_cursor_y = found_y;
                                let mut new_scroll_y = scroll_y;
                                if new_cursor_y < new_scroll_y
                                    || new_cursor_y >= new_scroll_y + edit_height
                                {
                                    new_scroll_y = new_cursor_y.saturating_sub(edit_height / 2);
                                }
                                state.active_popup = Some(PopupType::InternalEditor {
                                    path,
                                    lines,
                                    cursor_x: new_cursor_x,
                                    cursor_y: new_cursor_y,
                                    scroll_y: new_scroll_y,
                                    is_dirty,
                                    last_search: Some(q),
                                });
                            } else {
                                // Show "Text not found" popup message to satisfy the request.
                                state.active_popup =
                                    Some(PopupType::Error("Text not found".to_string()));
                            }
                        } else {
                            state.active_popup = Some(PopupType::InternalEditor {
                                path,
                                lines,
                                cursor_x,
                                cursor_y,
                                scroll_y,
                                is_dirty,
                                last_search,
                            });
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::EditorSearchPrompt {
                    path,
                    lines,
                    cursor_x,
                    cursor_y,
                    scroll_y,
                    is_dirty,
                    last_search,
                    query,
                });
                return Ok(None);
            }
            PopupType::InternalViewer { mut viewer } => {
                match key.code {
                    KeyCode::Esc | KeyCode::F(10) => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        viewer.scroll_up(1);
                    }
                    KeyCode::Down => {
                        viewer.scroll_down(1);
                    }
                    KeyCode::PageUp => {
                        viewer.scroll_up(18);
                    }
                    KeyCode::PageDown => {
                        viewer.scroll_down(18);
                    }
                    KeyCode::F(2) => {
                        viewer.toggle_mode();
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::InternalViewer { viewer });
                return Ok(None);
            }
            PopupType::Menu {
                active_menu_idx,
                active_item_idx,
            } => {
                let items = crate::ui::menu::get_menu_items(active_menu_idx);
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Left => {
                        let new_idx = if active_menu_idx > 0 {
                            active_menu_idx - 1
                        } else {
                            4
                        };
                        state.active_popup = Some(PopupType::Menu {
                            active_menu_idx: new_idx,
                            active_item_idx: 0,
                        });
                        return Ok(None);
                    }
                    KeyCode::Right => {
                        let new_idx = if active_menu_idx < 4 {
                            active_menu_idx + 1
                        } else {
                            0
                        };
                        state.active_popup = Some(PopupType::Menu {
                            active_menu_idx: new_idx,
                            active_item_idx: 0,
                        });
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        if !items.is_empty() {
                            let new_item_idx = if active_item_idx > 0 {
                                active_item_idx - 1
                            } else {
                                items.len() - 1
                            };
                            state.active_popup = Some(PopupType::Menu {
                                active_menu_idx,
                                active_item_idx: new_item_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Down => {
                        if !items.is_empty() {
                            let new_item_idx = if active_item_idx < items.len() - 1 {
                                active_item_idx + 1
                            } else {
                                0
                            };
                            state.active_popup = Some(PopupType::Menu {
                                active_menu_idx,
                                active_item_idx: new_item_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        state.active_popup = None;
                        let action =
                            trigger_menu_item(state, context, active_menu_idx, active_item_idx);
                        return Ok(action);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::DriveSelect {
                panel,
                ref drives,
                cursor_idx,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        if !drives.is_empty() {
                            let new_idx = if cursor_idx > 0 {
                                cursor_idx - 1
                            } else {
                                drives.len() - 1
                            };
                            state.active_popup = Some(PopupType::DriveSelect {
                                panel,
                                drives: drives.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Down => {
                        if !drives.is_empty() {
                            let new_idx = if cursor_idx < drives.len() - 1 {
                                cursor_idx + 1
                            } else {
                                0
                            };
                            state.active_popup = Some(PopupType::DriveSelect {
                                panel,
                                drives: drives.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if let Some(drive_path) = drives.get(cursor_idx) {
                            let target_path = std::path::PathBuf::from(drive_path);
                            match panel {
                                ActivePanel::Left => {
                                    state.left_panel.current_path = target_path;
                                    state.left_panel.cursor_index = 0;
                                    state.left_panel.selected_paths.clear();
                                }
                                ActivePanel::Right => {
                                    state.right_panel.current_path = target_path;
                                    state.right_panel.cursor_index = 0;
                                    state.right_panel.selected_paths.clear();
                                }
                            }
                            state.active_popup = None;
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::Hotlist {
                ref bookmarks,
                cursor_idx,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        if !bookmarks.is_empty() {
                            let new_idx = if cursor_idx > 0 {
                                cursor_idx - 1
                            } else {
                                bookmarks.len() - 1
                            };
                            state.active_popup = Some(PopupType::Hotlist {
                                bookmarks: bookmarks.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Down => {
                        if !bookmarks.is_empty() {
                            let new_idx = if cursor_idx < bookmarks.len() - 1 {
                                cursor_idx + 1
                            } else {
                                0
                            };
                            state.active_popup = Some(PopupType::Hotlist {
                                bookmarks: bookmarks.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if let Some((_, target_path)) = bookmarks.get(cursor_idx) {
                            let panel = state.get_active_panel_mut();
                            panel.current_path = target_path.clone();
                            panel.cursor_index = 0;
                            panel.selected_paths.clear();
                            state.active_popup = None;
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::RenMovPrompt {
                ref input,
                ref src_paths,
                ref dest_dir,
            } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::RenMovPrompt {
                            input: new_input,
                            src_paths: src_paths.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::RenMovPrompt {
                            input: new_input,
                            src_paths: src_paths.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let dest_dir = dest_dir.clone();
                        let src_paths = src_paths.clone();
                        let input = input.clone();
                        state.active_popup = None;

                        if src_paths.len() == 1 {
                            // Single item: use the input string as the new filename
                            let dst = dest_dir.join(&input);
                            if let Err(e) = crate::fs::rename_or_move_sync(&src_paths[0], &dst) {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Move failed: {}", e)));
                            }
                        } else {
                            // Multiple items: move all into dest_dir (ignore input as filename)
                            for src in &src_paths {
                                if let Some(fname) = src.file_name() {
                                    let dst = dest_dir.join(fname);
                                    if let Err(e) = crate::fs::rename_or_move_sync(src, &dst) {
                                        state.active_popup =
                                            Some(PopupType::Error(format!("Move failed: {}", e)));
                                        break;
                                    }
                                }
                            }
                        }

                        state.get_active_panel_mut().selected_paths.clear();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::SearchPrompt {
                ref query,
                ref content_query,
                ref search_root,
                focus_content,
            } => {
                match key.code {
                    KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                        state.active_popup = Some(PopupType::SearchPrompt {
                            query: query.clone(),
                            content_query: content_query.clone(),
                            search_root: search_root.clone(),
                            focus_content: !focus_content,
                        });
                        return Ok(None);
                    }
                    KeyCode::Char(c) => {
                        let mut new_query = query.clone();
                        let mut new_content = content_query.clone();
                        if focus_content {
                            new_content.push(c);
                        } else {
                            new_query.push(c);
                        }
                        state.active_popup = Some(PopupType::SearchPrompt {
                            query: new_query,
                            content_query: new_content,
                            search_root: search_root.clone(),
                            focus_content,
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_query = query.clone();
                        let mut new_content = content_query.clone();
                        if focus_content {
                            new_content.pop();
                        } else {
                            new_query.pop();
                        }
                        state.active_popup = Some(PopupType::SearchPrompt {
                            query: new_query,
                            content_query: new_content,
                            search_root: search_root.clone(),
                            focus_content,
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let q = query.clone();
                        let c_q = content_query.clone();
                        let search_root = search_root.clone();
                        if !q.is_empty() || !c_q.is_empty() {
                            let results = search_files_recursive(
                                &search_root,
                                &q,
                                if c_q.is_empty() { None } else { Some(&c_q) },
                            );
                            state.active_popup = Some(PopupType::SearchResults {
                                query: if q.is_empty() { c_q } else { q },
                                results,
                                cursor_idx: 0,
                            });
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::SearchResults {
                ref query,
                ref results,
                cursor_idx,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        if !results.is_empty() {
                            let new_idx = if cursor_idx > 0 {
                                cursor_idx - 1
                            } else {
                                results.len() - 1
                            };
                            state.active_popup = Some(PopupType::SearchResults {
                                query: query.clone(),
                                results: results.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Down => {
                        if !results.is_empty() {
                            let new_idx = if cursor_idx < results.len() - 1 {
                                cursor_idx + 1
                            } else {
                                0
                            };
                            state.active_popup = Some(PopupType::SearchResults {
                                query: query.clone(),
                                results: results.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if let Some(result_path) = results.get(cursor_idx) {
                            // Navigate the active panel to the directory containing the result
                            let target_dir = if result_path.is_dir() {
                                result_path.clone()
                            } else {
                                result_path
                                    .parent()
                                    .map(|p| p.to_path_buf())
                                    .unwrap_or_else(|| result_path.clone())
                            };
                            let panel = state.get_active_panel_mut();
                            panel.current_path = target_dir;
                            panel.cursor_index = 0;
                            panel.selected_paths.clear();
                            state.active_popup = None;
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::InfoPanel { .. } => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    state.active_popup = None;
                    return Ok(None);
                }
                return Err(());
            }
            PopupType::TreeView {
                ref nodes,
                cursor_idx,
                panel,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
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
                                nodes: nodes.clone(),
                                cursor_idx: new_idx,
                                panel,
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
                                nodes: nodes.clone(),
                                cursor_idx: new_idx,
                                panel,
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
                            match panel {
                                ActivePanel::Left => {
                                    state.left_panel.current_path = target;
                                    state.left_panel.cursor_index = 0;
                                    state.left_panel.selected_paths.clear();
                                }
                                ActivePanel::Right => {
                                    state.right_panel.current_path = target;
                                    state.right_panel.cursor_index = 0;
                                    state.right_panel.selected_paths.clear();
                                }
                            }
                            state.active_popup = None;
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::ContextMenu {
                ref items,
                cursor_idx,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        if !items.is_empty() {
                            let new_idx = if cursor_idx > 0 {
                                cursor_idx - 1
                            } else {
                                items.len() - 1
                            };
                            state.active_popup = Some(PopupType::ContextMenu {
                                items: items.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Down => {
                        if !items.is_empty() {
                            let new_idx = if cursor_idx < items.len() - 1 {
                                cursor_idx + 1
                            } else {
                                0
                            };
                            state.active_popup = Some(PopupType::ContextMenu {
                                items: items.clone(),
                                cursor_idx: new_idx,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if let Some(item) = items.get(cursor_idx) {
                            state.active_popup = None;
                            if item.contains("View") {
                                return Ok(Some(Action::View));
                            } else if item.contains("Edit") {
                                return Ok(Some(Action::Edit));
                            } else if item.contains("Copy") {
                                return Ok(Some(Action::Copy));
                            } else if item.contains("Move") {
                                return Ok(Some(Action::Move));
                            } else if item.contains("Delete") {
                                return Ok(Some(Action::Delete));
                            } else if item.contains("Compress") {
                                return Ok(Some(Action::CompressFiles));
                            } else if item.contains("Extract") {
                                return Ok(Some(Action::ExtractArchive));
                            }
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::CompressPrompt {
                ref input,
                ref targets,
                ref dest_dir,
            } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::CompressPrompt {
                            input: new_input,
                            targets: targets.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::CompressPrompt {
                            input: new_input,
                            targets: targets.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if !input.is_empty() {
                            let mut out_name = input.clone();
                            if !out_name.ends_with(".zip") {
                                out_name.push_str(".zip");
                            }
                            let final_dest = dest_dir.join(out_name);
                            let rx = crate::fs::spawn_compress_task(targets.clone(), final_dest);
                            state.progress_rx = Some(rx);
                            state.active_popup = Some(PopupType::CopyProgress {
                                current_file: "Compressing...".to_string(),
                                files_copied: 0,
                                total_files: 0,
                                bytes_copied: 0,
                                total_bytes: 0,
                            });
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::WipeConfirm { ref paths } => {
                match key.code {
                    KeyCode::Enter => {
                        let paths = paths.clone();
                        state.active_popup = None;
                        let rx = crate::fs::spawn_wipe_task(paths);
                        state.progress_rx = Some(rx);
                        state.active_popup = Some(PopupType::CopyProgress {
                            current_file: "Wiping...".to_string(),
                            files_copied: 0,
                            total_files: 0,
                            bytes_copied: 0,
                            total_bytes: 0,
                        });
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::SelectGroupPrompt {
                ref mode,
                ref query,
            } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_q = query.clone();
                        new_q.push(c);
                        state.active_popup = Some(PopupType::SelectGroupPrompt {
                            mode: mode.clone(),
                            query: new_q,
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_q = query.clone();
                        new_q.pop();
                        state.active_popup = Some(PopupType::SelectGroupPrompt {
                            mode: mode.clone(),
                            query: new_q,
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let mode = mode.clone();
                        let query = query.clone();
                        state.active_popup = None;
                        match mode {
                            crate::app::state::SelectMode::Add => {
                                state.get_active_panel_mut().select_group(&query)
                            }
                            crate::app::state::SelectMode::Remove => {
                                state.get_active_panel_mut().unselect_group(&query)
                            }
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::ApplyCommandPrompt {
                ref input,
                ref targets,
            } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::ApplyCommandPrompt {
                            input: new_input,
                            targets: targets.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::ApplyCommandPrompt {
                            input: new_input,
                            targets: targets.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let cmd = input.clone();
                        let targets = targets.clone();
                        state.active_popup = None;
                        if !cmd.is_empty() {
                            let rx = crate::fs::apply_command(cmd, targets);
                            state.progress_rx = Some(rx);
                            state.active_popup = Some(PopupType::CopyProgress {
                                current_file: "Running command...".to_string(),
                                files_copied: 0,
                                total_files: 0,
                                bytes_copied: 0,
                                total_bytes: 0,
                            });
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::DescribeFilePrompt {
                ref path,
                ref current_desc,
                ref input,
            } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::DescribeFilePrompt {
                            path: path.clone(),
                            current_desc: current_desc.clone(),
                            input: new_input,
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::DescribeFilePrompt {
                            path: path.clone(),
                            current_desc: current_desc.clone(),
                            input: new_input,
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let desc = input.clone();
                        let p = path.clone();
                        state.active_popup = None;
                        if let Some(dir) = p.parent() {
                            if let Some(name) = p.file_name() {
                                let _ = crate::fs::write_description(
                                    dir,
                                    &name.to_string_lossy(),
                                    &desc,
                                );
                            }
                        }
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::CreateLinkPrompt {
                ref src,
                ref dest_input,
                ref kind,
            } => {
                match key.code {
                    KeyCode::Char('s') | KeyCode::Char('h') => {
                        let new_kind = match key.code {
                            KeyCode::Char('s') => crate::app::state::LinkKind::Symbolic,
                            _ => crate::app::state::LinkKind::Hard,
                        };
                        state.active_popup = Some(PopupType::CreateLinkPrompt {
                            src: src.clone(),
                            dest_input: dest_input.clone(),
                            kind: new_kind,
                        });
                        return Ok(None);
                    }
                    KeyCode::Char(c) if !matches!(c, 's' | 'h') => {
                        let mut new_input = dest_input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::CreateLinkPrompt {
                            src: src.clone(),
                            dest_input: new_input,
                            kind: kind.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = dest_input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::CreateLinkPrompt {
                            src: src.clone(),
                            dest_input: new_input,
                            kind: kind.clone(),
                        });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let s = src.clone();
                        let kind = kind.clone();
                        let dest = state.get_passive_panel().current_path.join(dest_input);
                        state.active_popup = None;
                        let result = match kind {
                            crate::app::state::LinkKind::Symbolic => {
                                crate::fs::create_symlink(&s, &dest)
                            }
                            crate::app::state::LinkKind::Hard => {
                                crate::fs::create_hardlink(&s, &dest)
                            }
                        };
                        if let Err(e) = result {
                            state.active_popup =
                                Some(PopupType::Error(format!("Link failed: {}", e)));
                        } else {
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::FilePanelFilterPrompt { ref input } => {
                match key.code {
                    KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup =
                            Some(PopupType::FilePanelFilterPrompt { input: new_input });
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup =
                            Some(PopupType::FilePanelFilterPrompt { input: new_input });
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        let mask = input.trim().to_string();
                        state.active_popup = None;
                        let panel = state.get_active_panel_mut();
                        panel.filter_mask = if mask.is_empty() { None } else { Some(mask) };
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::TaskListDialog {
                mut tasks,
                mut cursor_idx,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Up => {
                        if cursor_idx > 0 {
                            cursor_idx -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if !tasks.is_empty() && cursor_idx < tasks.len().saturating_sub(1) {
                            cursor_idx += 1;
                        }
                    }
                    KeyCode::Delete | KeyCode::Char('k') => {
                        if let Some(task) = tasks.get(cursor_idx) {
                            let pid = task.pid;
                            match kill_process(pid) {
                                Ok(_) => {
                                    tasks.remove(cursor_idx);
                                    if cursor_idx >= tasks.len() && cursor_idx > 0 {
                                        cursor_idx = tasks.len().saturating_sub(1);
                                    }
                                }
                                Err(e) => {
                                    state.active_popup = Some(PopupType::Error(format!(
                                        "Failed to kill process: {}",
                                        e
                                    )));
                                    return Ok(None);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::TaskListDialog { tasks, cursor_idx });
                return Ok(None);
            }
            // Dismiss-only popups for new types not yet fully interactive
            PopupType::SortModesDialog { .. }
            | PopupType::CompareFoldersResult { .. }
            | PopupType::FileAssociationsDialog { .. }
            | PopupType::ArchiveCommandsMenu { .. }
            | PopupType::QuickViewPanel { .. }
            | PopupType::CommandHistoryList { .. }
            | PopupType::FileViewHistoryList { .. }
            | PopupType::FoldersHistoryList { .. } => {
                if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                    state.active_popup = None;
                    return Ok(None);
                }
                return Err(());
            }
            PopupType::SaveSetupConfirm => {
                match key.code {
                    KeyCode::Enter => {
                        match context.config.save() {
                            Ok(_) => {
                                state.active_popup = Some(PopupType::Info(
                                    "Configuration saved successfully.".to_string(),
                                ));
                            }
                            Err(e) => {
                                state.active_popup =
                                    Some(PopupType::Error(format!("Failed to save setup: {}", e)));
                            }
                        }
                        return Ok(None);
                    }
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Ok(None);
            }
            PopupType::ConfigurationDialog {
                mut active_tab,
                mut cursor_idx,
                mut editing_value,
                mut edit_buffer,
                mut settings,
            } => {
                let max_rows = match active_tab {
                    0 => 19, // System (17 settings + 2 buttons)
                    1 => 35, // Panel (33 settings + 2 buttons)
                    2 => 39, // Interface (37 settings + 2 buttons)
                    3 => 16, // Confirmations (14 settings + 2 buttons)
                    4 => 13, // Language & Plugins (11 settings + 2 buttons)
                    5 => 41, // Editor/Viewer (39 settings + 2 buttons)
                    6 => 5,  // Colors (3 settings + 2 buttons)
                    _ => 5,
                };

                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                if editing_value {
                    match key.code {
                        KeyCode::Char(c) if !is_ctrl => {
                            edit_buffer.push(c);
                        }
                        KeyCode::Backspace => {
                            edit_buffer.pop();
                        }
                        KeyCode::Esc => {
                            editing_value = false;
                        }
                        KeyCode::Enter => {
                            if active_tab == 5 && cursor_idx == 1 {
                                settings.default_editor = edit_buffer.clone();
                            } else if active_tab == 5 && cursor_idx == 22 {
                                settings.viewer_command = edit_buffer.clone();
                            } else if active_tab == 2 && cursor_idx == 14 {
                                settings.interface_window_title_addons = edit_buffer.clone();
                            }
                            editing_value = false;
                        }
                        _ => {}
                    }
                    state.active_popup = Some(PopupType::ConfigurationDialog {
                        active_tab,
                        cursor_idx,
                        editing_value,
                        edit_buffer,
                        settings,
                    });
                    return Ok(None);
                }

                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Left => {
                        if active_tab > 0 {
                            active_tab -= 1;
                        } else {
                            active_tab = 6;
                        }
                        cursor_idx = 0;
                    }
                    KeyCode::Right => {
                        if active_tab < 6 {
                            active_tab += 1;
                        } else {
                            active_tab = 0;
                        }
                        cursor_idx = 0;
                    }
                    KeyCode::Up => {
                        if cursor_idx > 0 {
                            cursor_idx -= 1;
                        } else {
                            cursor_idx = max_rows - 1;
                        }
                    }
                    KeyCode::Down => {
                        if cursor_idx < max_rows - 1 {
                            cursor_idx += 1;
                        } else {
                            cursor_idx = 0;
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        let ok_idx = max_rows - 2;
                        let cancel_idx = max_rows - 1;

                        if cursor_idx == ok_idx {
                            if settings.theme != context.config.settings.theme {
                                change_theme(context, state, &settings.theme);
                            }
                            if settings.keybinding_preset
                                != context.config.settings.keybinding_preset
                            {
                                change_preset(context, &settings.keybinding_preset);
                            }
                            context.config.settings = settings;
                            let _ = context.config.save();
                            state.refresh_both_panels(context.config.settings.show_hidden);
                            state.active_popup = None;
                            return Ok(None);
                        } else if cursor_idx == cancel_idx {
                            state.active_popup = None;
                            return Ok(None);
                        }

                        match active_tab {
                            0 => match cursor_idx {
                                0 => {
                                    settings.delete_to_recycle_bin = !settings.delete_to_recycle_bin
                                }
                                1 => {
                                    settings.use_system_copy_routine =
                                        !settings.use_system_copy_routine
                                }
                                2 => {
                                    settings.copy_files_opened_for_writing =
                                        !settings.copy_files_opened_for_writing
                                }
                                3 => settings.scan_symbolic_links = !settings.scan_symbolic_links,
                                4 => {
                                    settings.save_commands_history = !settings.save_commands_history
                                }
                                5 => settings.save_folders_history = !settings.save_folders_history,
                                6 => {
                                    settings.save_view_and_edit_history =
                                        !settings.save_view_and_edit_history
                                }
                                7 => {
                                    settings.use_windows_registered_types =
                                        !settings.use_windows_registered_types
                                }
                                8 => {
                                    settings.automatic_update_env_variables =
                                        !settings.automatic_update_env_variables
                                }
                                10 => {
                                    settings.req_admin_modification =
                                        !settings.req_admin_modification
                                }
                                11 => settings.req_admin_reading = !settings.req_admin_reading,
                                12 => {
                                    settings.req_admin_use_additional_privileges =
                                        !settings.req_admin_use_additional_privileges
                                }
                                13 => {
                                    settings.sorting_collation =
                                        match settings.sorting_collation.as_str() {
                                            "linguistic" => "natural".to_string(),
                                            _ => "linguistic".to_string(),
                                        };
                                }
                                14 => {
                                    settings.treat_digits_as_numbers =
                                        !settings.treat_digits_as_numbers
                                }
                                15 => settings.case_sensitive_sort = !settings.case_sensitive_sort,
                                16 => settings.auto_save_setup = !settings.auto_save_setup,
                                _ => {}
                            },
                            1 => match cursor_idx {
                                0 => settings.show_hidden = !settings.show_hidden,
                                1 => settings.highlight_files = !settings.highlight_files,
                                2 => settings.select_folders = !settings.select_folders,
                                3 => {
                                    settings.right_click_selects_files =
                                        !settings.right_click_selects_files
                                }
                                4 => {
                                    settings.sort_folder_names_by_extension =
                                        !settings.sort_folder_names_by_extension
                                }
                                5 => settings.sort_reverse = !settings.sort_reverse,
                                6 => {
                                    settings.disable_panel_update_object_count =
                                        match settings.disable_panel_update_object_count {
                                            0 => 100,
                                            100 => 1000,
                                            1000 => 10000,
                                            _ => 0,
                                        };
                                }
                                7 => {
                                    settings.network_drives_autorefresh =
                                        !settings.network_drives_autorefresh
                                }
                                8 => settings.show_column_titles = !settings.show_column_titles,
                                9 => settings.show_status_line = !settings.show_status_line,
                                10 => {
                                    settings.detect_volume_mount_points =
                                        !settings.detect_volume_mount_points
                                }
                                11 => {
                                    settings.show_files_total_information =
                                        !settings.show_files_total_information
                                }
                                12 => settings.show_free_size = !settings.show_free_size,
                                13 => settings.show_scrollbar = !settings.show_scrollbar,
                                14 => {
                                    settings.show_background_screens_number =
                                        !settings.show_background_screens_number
                                }
                                15 => {
                                    settings.show_sort_mode_letter = !settings.show_sort_mode_letter
                                }
                                16 => {
                                    settings.show_dotdot_in_root_folders =
                                        !settings.show_dotdot_in_root_folders
                                }
                                18 => {
                                    settings.infopanel_show_power_status =
                                        !settings.infopanel_show_power_status
                                }
                                19 => {
                                    settings.infopanel_show_cd_drive_parameters =
                                        !settings.infopanel_show_cd_drive_parameters
                                }
                                20 => {
                                    settings.infopanel_computer_name_format =
                                        match settings.infopanel_computer_name_format.as_str() {
                                            "Physical NetBIOS" => "DNS name".to_string(),
                                            _ => "Physical NetBIOS".to_string(),
                                        };
                                }
                                21 => {
                                    settings.infopanel_user_name_format =
                                        match settings.infopanel_user_name_format.as_str() {
                                            "Logon name" => "UPN".to_string(),
                                            _ => "Logon name".to_string(),
                                        };
                                }
                                25 => {
                                    settings.file_descriptions_list_names =
                                        match settings.file_descriptions_list_names.as_str() {
                                            "Descript.ion,Files.bbs" => "descript.ion".to_string(),
                                            "descript.ion" => "files.bbs".to_string(),
                                            _ => "Descript.ion,Files.bbs".to_string(),
                                        };
                                }
                                26 => {
                                    settings.file_descriptions_set_hidden =
                                        !settings.file_descriptions_set_hidden
                                }
                                27 => {
                                    settings.file_descriptions_update_readonly =
                                        !settings.file_descriptions_update_readonly
                                }
                                28 => {
                                    settings.file_descriptions_position =
                                        match settings.file_descriptions_position {
                                            0 => 1,
                                            1 => 2,
                                            _ => 0,
                                        };
                                }
                                29 => {
                                    settings.file_descriptions_update_mode =
                                        match settings.file_descriptions_update_mode.as_str() {
                                            "Do not update" => "Update if displayed".to_string(),
                                            "Update if displayed" => "Always update".to_string(),
                                            _ => "Do not update".to_string(),
                                        };
                                }
                                30 => {
                                    settings.file_descriptions_use_ansi =
                                        !settings.file_descriptions_use_ansi
                                }
                                31 => {
                                    settings.file_descriptions_save_utf8 =
                                        !settings.file_descriptions_save_utf8
                                }
                                32 => {
                                    settings.folder_description_list_names = match settings
                                        .folder_description_list_names
                                        .as_str()
                                    {
                                        "DirInfo,File_Id.diz,Descript.ion,ReadMe.*,Read.Me" => {
                                            "DirInfo,File_Id.diz".to_string()
                                        }
                                        _ => "DirInfo,File_Id.diz,Descript.ion,ReadMe.*,Read.Me"
                                            .to_string(),
                                    };
                                }
                                _ => {}
                            },
                            2 => match cursor_idx {
                                0 => settings.interface_clock = !settings.interface_clock,
                                1 => settings.mouse_support = !settings.mouse_support,
                                2 => {
                                    settings.interface_show_key_bar =
                                        !settings.interface_show_key_bar
                                }
                                3 => {
                                    settings.interface_always_show_menu_bar =
                                        !settings.interface_always_show_menu_bar
                                }
                                4 => {
                                    settings.interface_screen_saver_minutes =
                                        match settings.interface_screen_saver_minutes {
                                            1 => 5,
                                            5 => 10,
                                            10 => 15,
                                            15 => 30,
                                            30 => 60,
                                            _ => 1,
                                        };
                                }
                                5 => {
                                    settings.interface_show_total_copy_progress =
                                        !settings.interface_show_total_copy_progress
                                }
                                6 => {
                                    settings.interface_show_copying_time =
                                        !settings.interface_show_copying_time
                                }
                                7 => {
                                    settings.interface_show_total_delete_progress =
                                        !settings.interface_show_total_delete_progress
                                }
                                8 => {
                                    settings.interface_use_ctrl_pgup_change_drive =
                                        !settings.interface_use_ctrl_pgup_change_drive
                                }
                                9 => {
                                    settings.interface_use_virtual_terminal =
                                        !settings.interface_use_virtual_terminal
                                }
                                10 => {
                                    settings.interface_fullwidth_aware_rendering =
                                        !settings.interface_fullwidth_aware_rendering
                                }
                                11 => {
                                    settings.interface_cleartype_friendly_redraw =
                                        !settings.interface_cleartype_friendly_redraw
                                }
                                12 => {
                                    settings.interface_console_icon =
                                        match settings.interface_console_icon {
                                            0 => 1,
                                            1 => 2,
                                            _ => 0,
                                        };
                                }
                                13 => {
                                    settings.interface_console_icon_admin_alternate =
                                        !settings.interface_console_icon_admin_alternate
                                }
                                14 => {
                                    editing_value = true;
                                    edit_buffer = settings.interface_window_title_addons.clone();
                                }
                                16 => {
                                    settings.dialog_history_in_edit_controls =
                                        !settings.dialog_history_in_edit_controls
                                }
                                17 => {
                                    settings.dialog_persistent_blocks =
                                        !settings.dialog_persistent_blocks
                                }
                                18 => {
                                    settings.dialog_del_removes_blocks =
                                        !settings.dialog_del_removes_blocks
                                }
                                19 => settings.dialog_autocomplete = !settings.dialog_autocomplete,
                                20 => {
                                    settings.dialog_backspace_deletes_unchanged =
                                        !settings.dialog_backspace_deletes_unchanged
                                }
                                21 => {
                                    settings.dialog_mouse_click_outside_closes =
                                        !settings.dialog_mouse_click_outside_closes
                                }
                                23 => {
                                    settings.menu_left_click_outside =
                                        match settings.menu_left_click_outside.as_str() {
                                            "Cancel menu" => "Do nothing".to_string(),
                                            _ => "Cancel menu".to_string(),
                                        };
                                }
                                24 => {
                                    settings.menu_right_click_outside =
                                        match settings.menu_right_click_outside.as_str() {
                                            "Cancel menu" => "Do nothing".to_string(),
                                            _ => "Cancel menu".to_string(),
                                        };
                                }
                                25 => {
                                    settings.menu_middle_click_outside =
                                        match settings.menu_middle_click_outside.as_str() {
                                            "Execute selected item" => "Cancel menu".to_string(),
                                            _ => "Execute selected item".to_string(),
                                        };
                                }
                                27 => {
                                    settings.cmdline_persistent_blocks =
                                        !settings.cmdline_persistent_blocks
                                }
                                28 => {
                                    settings.cmdline_del_removes_blocks =
                                        !settings.cmdline_del_removes_blocks
                                }
                                29 => {
                                    settings.cmdline_autocomplete = !settings.cmdline_autocomplete
                                }
                                30 => {
                                    settings.cmdline_prompt_format =
                                        match settings.cmdline_prompt_format.as_str() {
                                            "$p$g" => "$p".to_string(),
                                            "$p" => "$g".to_string(),
                                            _ => "$p$g".to_string(),
                                        };
                                }
                                31 => {
                                    settings.cmdline_use_home_dir =
                                        match settings.cmdline_use_home_dir.as_str() {
                                            "%FARHOME%" => "%USERPROFILE%".to_string(),
                                            _ => "%FARHOME%".to_string(),
                                        };
                                }
                                33 => {
                                    settings.autocomplete_show_list =
                                        !settings.autocomplete_show_list
                                }
                                34 => {
                                    settings.autocomplete_modal_mode =
                                        !settings.autocomplete_modal_mode
                                }
                                35 => {
                                    settings.autocomplete_append_first =
                                        !settings.autocomplete_append_first
                                }
                                36 => {
                                    settings.keybinding_preset =
                                        match settings.keybinding_preset.as_str() {
                                            "norton" => "vim".to_string(),
                                            "vim" => "modern".to_string(),
                                            _ => "norton".to_string(),
                                        };
                                }
                                _ => {}
                            },
                            3 => match cursor_idx {
                                0 => {
                                    settings.confirmations.confirm_copy =
                                        !settings.confirmations.confirm_copy
                                }
                                1 => {
                                    settings.confirmations.confirm_move =
                                        !settings.confirmations.confirm_move
                                }
                                2 => {
                                    settings.confirmations.confirm_overwrite =
                                        !settings.confirmations.confirm_overwrite
                                }
                                3 => {
                                    settings.confirmations.confirm_drag_and_drop =
                                        !settings.confirmations.confirm_drag_and_drop
                                }
                                4 => {
                                    settings.confirmations.confirm_delete =
                                        !settings.confirmations.confirm_delete
                                }
                                5 => {
                                    settings.confirmations.confirm_delete_non_empty_folders =
                                        !settings.confirmations.confirm_delete_non_empty_folders
                                }
                                6 => {
                                    settings.confirmations.confirm_interrupt_operation =
                                        !settings.confirmations.confirm_interrupt_operation
                                }
                                7 => {
                                    settings.confirmations.confirm_disconnect_network_drive =
                                        !settings.confirmations.confirm_disconnect_network_drive
                                }
                                8 => {
                                    settings.confirmations.confirm_delete_subst_disk =
                                        !settings.confirmations.confirm_delete_subst_disk
                                }
                                9 => {
                                    settings.confirmations.confirm_detach_virtual_disk =
                                        !settings.confirmations.confirm_detach_virtual_disk
                                }
                                10 => {
                                    settings.confirmations.confirm_hotplug_removal =
                                        !settings.confirmations.confirm_hotplug_removal
                                }
                                11 => {
                                    settings.confirmations.confirm_reload_edited_file =
                                        !settings.confirmations.confirm_reload_edited_file
                                }
                                12 => {
                                    settings.confirmations.confirm_clear_history_list =
                                        !settings.confirmations.confirm_clear_history_list
                                }
                                13 => {
                                    settings.confirmations.confirm_quit =
                                        !settings.confirmations.confirm_quit
                                }
                                _ => {}
                            },
                            4 => match cursor_idx {
                                0 => {
                                    settings.language = match settings.language.as_str() {
                                        "English" => "Spanish".to_string(),
                                        _ => "English".to_string(),
                                    };
                                }
                                3 => {
                                    settings.plugins_manager_oem_support =
                                        !settings.plugins_manager_oem_support
                                }
                                4 => {
                                    settings.plugins_manager_scan_symlinks =
                                        !settings.plugins_manager_scan_symlinks
                                }
                                6 => {
                                    settings.plugins_manager_file_processing =
                                        !settings.plugins_manager_file_processing
                                }
                                7 => {
                                    settings.plugins_manager_show_standard_association =
                                        !settings.plugins_manager_show_standard_association
                                }
                                8 => {
                                    settings.plugins_manager_even_if_one_found =
                                        !settings.plugins_manager_even_if_one_found
                                }
                                9 => {
                                    settings.plugins_manager_search_results =
                                        !settings.plugins_manager_search_results
                                }
                                10 => {
                                    settings.plugins_manager_prefix_processing =
                                        !settings.plugins_manager_prefix_processing
                                }
                                _ => {}
                            },
                            5 => match cursor_idx {
                                0 => settings.editor_use_external = !settings.editor_use_external,
                                1 => {
                                    editing_value = true;
                                    edit_buffer = settings.default_editor.clone();
                                }
                                3 => {
                                    settings.editor_expand_tabs =
                                        match settings.editor_expand_tabs.as_str() {
                                            "Do not expand tabs" => "Expand tabs".to_string(),
                                            _ => "Do not expand tabs".to_string(),
                                        };
                                }
                                4 => {
                                    settings.editor_persistent_blocks =
                                        !settings.editor_persistent_blocks
                                }
                                5 => {
                                    settings.editor_cursor_beyond_eol =
                                        !settings.editor_cursor_beyond_eol
                                }
                                6 => {
                                    settings.editor_del_removes_blocks =
                                        !settings.editor_del_removes_blocks
                                }
                                7 => settings.editor_select_found = !settings.editor_select_found,
                                8 => settings.editor_auto_indent = !settings.editor_auto_indent,
                                9 => settings.editor_cursor_at_end = !settings.editor_cursor_at_end,
                                10 => {
                                    settings.editor_tab_size = match settings.editor_tab_size {
                                        2 => 4,
                                        4 => 8,
                                        _ => 2,
                                    };
                                }
                                11 => {
                                    settings.editor_show_scrollbar = !settings.editor_show_scrollbar
                                }
                                12 => {
                                    settings.editor_show_white_space =
                                        !settings.editor_show_white_space
                                }
                                13 => {
                                    settings.editor_show_line_numbers =
                                        !settings.editor_show_line_numbers
                                }
                                14 => {
                                    settings.editor_save_file_position =
                                        !settings.editor_save_file_position
                                }
                                15 => {
                                    settings.editor_save_bookmarks = !settings.editor_save_bookmarks
                                }
                                16 => {
                                    settings.editor_allow_editing_opened_writing =
                                        !settings.editor_allow_editing_opened_writing
                                }
                                17 => {
                                    settings.editor_lock_editing_readonly =
                                        !settings.editor_lock_editing_readonly
                                }
                                18 => {
                                    settings.editor_warn_opening_readonly =
                                        !settings.editor_warn_opening_readonly
                                }
                                19 => {
                                    settings.editor_autodetect_codepage =
                                        !settings.editor_autodetect_codepage
                                }
                                20 => {
                                    settings.editor_default_codepage =
                                        match settings.editor_default_codepage.as_str() {
                                            "1252" => "65001".to_string(),
                                            "65001" => "1200".to_string(),
                                            _ => "1252".to_string(),
                                        };
                                }
                                21 => settings.viewer_use_external = !settings.viewer_use_external,
                                22 => {
                                    editing_value = true;
                                    edit_buffer = settings.viewer_command.clone();
                                }
                                24 => {
                                    settings.viewer_persistent_selection =
                                        !settings.viewer_persistent_selection
                                }
                                25 => {
                                    settings.viewer_show_scrolling_arrows =
                                        !settings.viewer_show_scrolling_arrows
                                }
                                26 => {
                                    settings.viewer_tab_size = match settings.viewer_tab_size {
                                        2 => 4,
                                        4 => 8,
                                        _ => 2,
                                    };
                                }
                                27 => settings.viewer_visible_zero = !settings.viewer_visible_zero,
                                28 => {
                                    settings.viewer_show_scrollbar = !settings.viewer_show_scrollbar
                                }
                                29 => {
                                    settings.viewer_save_file_position =
                                        !settings.viewer_save_file_position
                                }
                                30 => {
                                    settings.viewer_save_view_mode = !settings.viewer_save_view_mode
                                }
                                31 => {
                                    settings.viewer_save_file_codepage =
                                        !settings.viewer_save_file_codepage
                                }
                                32 => {
                                    settings.viewer_save_wrap_mode = !settings.viewer_save_wrap_mode
                                }
                                33 => {
                                    settings.viewer_save_bookmarks = !settings.viewer_save_bookmarks
                                }
                                34 => {
                                    settings.viewer_detect_dump_view_mode =
                                        !settings.viewer_detect_dump_view_mode
                                }
                                35 => {
                                    settings.viewer_max_line_width =
                                        match settings.viewer_max_line_width {
                                            1000 => 10000,
                                            10000 => 50000,
                                            _ => 1000,
                                        };
                                }
                                36 => {
                                    settings.viewer_autodetect_codepage =
                                        !settings.viewer_autodetect_codepage
                                }
                                37 => {
                                    settings.viewer_default_codepage =
                                        match settings.viewer_default_codepage.as_str() {
                                            "1252" => "65001".to_string(),
                                            "65001" => "1200".to_string(),
                                            _ => "1252".to_string(),
                                        };
                                }
                                _ => {}
                            },
                            6 => match cursor_idx {
                                0 => {
                                    settings.theme = match settings.theme.as_str() {
                                        "slate" => "classic_blue".to_string(),
                                        _ => "slate".to_string(),
                                    };
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    KeyCode::F(9) => {
                        if settings.theme != context.config.settings.theme {
                            change_theme(context, state, &settings.theme);
                        }
                        if settings.keybinding_preset != context.config.settings.keybinding_preset {
                            change_preset(context, &settings.keybinding_preset);
                        }
                        context.config.settings = settings;
                        let _ = context.config.save();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }

                state.active_popup = Some(PopupType::ConfigurationDialog {
                    active_tab,
                    cursor_idx,
                    editing_value,
                    edit_buffer,
                    settings,
                });
                return Ok(None);
            }
            PopupType::FileAttributesDialog {
                mut attrs,
                mut mode_input,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    KeyCode::Char(c) if c.is_digit(8) => {
                        if mode_input.len() < 4 {
                            mode_input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        mode_input.pop();
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char(' ') => {
                        attrs.readonly = !attrs.readonly;
                    }
                    KeyCode::Enter => {
                        if !mode_input.is_empty() {
                            if let Ok(mode) = u32::from_str_radix(&mode_input, 8) {
                                if let Err(e) = crate::fs::attrs::set_unix_mode(&attrs.path, mode) {
                                    state.active_popup = Some(PopupType::Error(format!(
                                        "Failed to set unix mode: {}",
                                        e
                                    )));
                                    return Ok(None);
                                }
                            }
                        }
                        if let Err(e) = crate::fs::attrs::set_readonly(&attrs.path, attrs.readonly)
                        {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to set readonly: {}", e)));
                            return Ok(None);
                        }
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                state.active_popup = Some(PopupType::FileAttributesDialog { attrs, mode_input });
                return Ok(None);
            }
        }
    }
    Err(())
}
