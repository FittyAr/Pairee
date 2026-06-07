use super::context::AppContext;
use super::state::{ActivePanel, AppState, PopupType, TreeNode};
use crate::keybindings::Action;
use crate::terminal::{Event, EventHandler, TerminalBackend};
use crate::ui;
use anyhow::Result;
use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::path::Path;
use std::time::Duration;

/// Runs the main loop for NCRust.
pub async fn run(mut context: AppContext, mut state: AppState) -> Result<()> {
    let mut terminal_backend = TerminalBackend::init()?;
    let mut event_handler = EventHandler::new(Duration::from_millis(50));

    // Initial folder scans
    state.refresh_both_panels(context.config.settings.show_hidden);

    loop {
        // 1. Process background operation updates (e.g. copy progress)
        if state.progress_rx.is_some() {
            let mut rx = state.progress_rx.take().unwrap();
            let mut is_completed = false;
            let mut has_error = None;
            let mut latest_update = None;

            while let Ok(update) = rx.try_recv() {
                if let Some(err) = update.error.clone() {
                    has_error = Some(err);
                } else if update.current_file == "Completed" {
                    is_completed = true;
                } else {
                    latest_update = Some(update);
                }
            }

            if let Some(err) = has_error {
                state.active_popup = Some(PopupType::Error(err));
            } else if is_completed {
                state.active_popup = None;
                state.refresh_both_panels(context.config.settings.show_hidden);
            } else {
                if let Some(update) = latest_update {
                    state.active_popup = Some(PopupType::CopyProgress {
                        current_file: update.current_file,
                        files_copied: update.files_copied,
                        total_files: update.total_files,
                        bytes_copied: update.bytes_copied,
                        total_bytes: update.total_bytes,
                    });
                }
                state.progress_rx = Some(rx);
            }
        }

        // 2. Draw terminal window
        terminal_backend.terminal.draw(|f| {
            ui::draw_ui(f, &context, &state);
        })?;

        // 3. Exit check
        if state.should_quit {
            break;
        }

        // 4. Handle input events
        if let Some(event) = event_handler.next().await {
            match event {
                Event::Key(key) => {
                    // Filter out KeyRelease events on Windows to prevent double-step triggers
                    if key.kind == crossterm::event::KeyEventKind::Release {
                        continue;
                    }

                    // Popups consume inputs first
                    match handle_popup_input(&mut state, key, &mut context) {
                        Ok(Some(action)) => {
                            handle_action(&mut state, action, &mut context, &mut terminal_backend)
                                .await?;
                            continue;
                        }
                        Ok(None) => {
                            continue;
                        }
                        Err(()) => {}
                    }

                    // CLI input takes priority next if applicable
                    if handle_cli_input(&mut state, key, &context, &mut terminal_backend).is_ok() {
                        continue;
                    }

                    // Standard resolved actions
                    if let Some(action) = context.resolver.resolve(key) {
                        handle_action(&mut state, action, &mut context, &mut terminal_backend)
                            .await?;
                    }
                }
                Event::Resize(_, _) => {
                    // Ratatui auto-redraws next iteration
                }
                Event::Tick => {}
                _ => {}
            }
        }
    }

    Ok(())
}

/// Dispatches actions to their respective state changes.
async fn handle_action(
    state: &mut AppState,
    action: Action,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    match action {
        Action::MoveUp => {
            state.get_active_panel_mut().move_cursor_up();
        }
        Action::MoveDown => {
            state.get_active_panel_mut().move_cursor_down();
        }
        Action::PageUp => {
            state.get_active_panel_mut().page_up(10);
        }
        Action::PageDown => {
            state.get_active_panel_mut().page_down(10);
        }
        Action::GoToTop => {
            state.get_active_panel_mut().go_to_top();
        }
        Action::GoToBottom => {
            state.get_active_panel_mut().go_to_bottom();
        }
        Action::ChangePanel => {
            state.toggle_focus();
        }
        Action::SelectItem => {
            state.get_active_panel_mut().toggle_selection();
            state.get_active_panel_mut().move_cursor_down();
        }
        Action::Execute => {
            handle_enter_key(state, context.config.settings.show_hidden);
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Action::GoParent => {
            handle_backspace_key(state, context.config.settings.show_hidden);
        }
        Action::Help => {
            state.active_popup = Some(PopupType::Help);
        }
        Action::UserMenu => {
            state.active_popup = Some(PopupType::UserMenu);
        }
        Action::View => {
            let active = state.get_active_panel();
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
                let pager = if cfg!(target_os = "windows") {
                    "more"
                } else {
                    "less"
                };
                if let Err(e) =
                    execute_external_command(&entry.path.clone(), pager, terminal_backend)
                {
                    state.active_popup =
                        Some(PopupType::Error(format!("View failed: {}", e)));
                }
            }
        }
        Action::Edit => {
            let active = state.get_active_panel();
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
                let path = entry.path.clone();
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        state.active_popup = Some(PopupType::InternalEditor {
                            path,
                            lines: if lines.is_empty() {
                                vec![String::new()]
                            } else {
                                lines
                            },
                            cursor_x: 0,
                            cursor_y: 0,
                            scroll_y: 0,
                            is_dirty: false,
                        });
                    }
                    Err(e) => {
                        state.active_popup =
                            Some(PopupType::Error(format!("Cannot read file: {}", e)));
                    }
                }
            }
        }
        Action::Copy => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let dest = state.get_passive_panel().current_path.clone();
                let rx = crate::fs::spawn_copy_task(targets, dest);
                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    current_file: "Initializing...".to_string(),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
                });
            }
        }
        Action::Move => {
            // Open a rename/move prompt so user can confirm/edit destination
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let dest_dir = state.get_passive_panel().current_path.clone();
                // Pre-fill input with the destination filename of the first item
                let default_input = targets
                    .first()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                state.active_popup = Some(PopupType::RenMovPrompt {
                    input: default_input,
                    src_paths: targets,
                    dest_dir,
                });
            }
        }
        Action::MkDir => {
            state.active_popup = Some(PopupType::MkDirPrompt {
                input: String::new(),
            });
        }
        Action::Delete => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                state.active_popup = Some(PopupType::ConfirmDelete { paths: targets });
            }
        }
        Action::Menu => {
            if let Some(PopupType::Menu { .. }) = state.active_popup {
                state.active_popup = None;
            } else {
                state.active_popup = Some(PopupType::Menu {
                    active_menu_idx: 0,
                    active_item_idx: 0,
                });
            }
        }
        Action::Quit => {
            state.should_quit = true;
        }
        Action::ToggleHidden => {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            let _ = context.config.save();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Action::FocusCli => {
            state.cli_input.push(' ');
            state.cli_input.clear();
        }
        Action::Unfocus => {
            state.active_popup = None;
            state.cli_input.clear();
        }
        Action::Refresh => {
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Action::SwapPanels => {
            state.swap_panels();
        }
        Action::DriveSelectLeft => {
            let drives = get_system_drives();
            state.active_popup = Some(PopupType::DriveSelect {
                panel: ActivePanel::Left,
                drives,
                cursor_idx: 0,
            });
        }
        Action::DriveSelectRight => {
            let drives = get_system_drives();
            state.active_popup = Some(PopupType::DriveSelect {
                panel: ActivePanel::Right,
                drives,
                cursor_idx: 0,
            });
        }
    }
    Ok(())
}

/// Captures keyboard input for active popups.
fn handle_popup_input(
    state: &mut AppState,
    key: crossterm::event::KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let popup = state.active_popup.clone();
    if let Some(p) = popup {
        match p {
            PopupType::MkDirPrompt { ref input } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::MkDirPrompt { input: new_input });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::MkDirPrompt { input: new_input });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::ConfirmDelete { ref paths } => {
                match key.code {
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::Error(_) | PopupType::Help | PopupType::Info(_) => {
                if key.code == crossterm::event::KeyCode::Esc
                    || key.code == crossterm::event::KeyCode::Enter
                {
                    state.active_popup = None;
                    return Ok(None);
                }
                return Err(());
            }
            PopupType::CopyProgress { .. } => {
                if key.code == crossterm::event::KeyCode::Esc {
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
                    crossterm::event::KeyCode::Char('1') => {
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char('2') => {
                        context.config.settings.show_hidden = !context.config.settings.show_hidden;
                        let _ = context.config.save();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char('3') => {
                        state.swap_panels();
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char('4') => {
                        state.active_popup = Some(PopupType::Help);
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char('5') | crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
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
            } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
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
                    crossterm::event::KeyCode::Backspace => {
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
                    crossterm::event::KeyCode::Delete => {
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
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Up => {
                        if cursor_y > 0 {
                            cursor_y -= 1;
                            cursor_x = cursor_x.min(lines[cursor_y].len());
                            if cursor_y < scroll_y {
                                scroll_y = cursor_y;
                            }
                        }
                    }
                    crossterm::event::KeyCode::Down => {
                        if cursor_y < lines.len().saturating_sub(1) {
                            cursor_y += 1;
                            cursor_x = cursor_x.min(lines[cursor_y].len());
                            if cursor_y >= scroll_y + 18 {
                                scroll_y = cursor_y - 17;
                            }
                        }
                    }
                    crossterm::event::KeyCode::Left => {
                        if cursor_x > 0 {
                            cursor_x -= 1;
                        } else if cursor_y > 0 {
                            cursor_y -= 1;
                            cursor_x = lines[cursor_y].len();
                        }
                    }
                    crossterm::event::KeyCode::Right => {
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
                    crossterm::event::KeyCode::F(2) => {
                        let content = lines.join("\n");
                        if let Err(e) = std::fs::write(&path, content) {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to save: {}", e)));
                            return Ok(None);
                        }
                        is_dirty = false;
                    }
                    crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::F(10) => {
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
                });
                return Ok(None);
            }
            PopupType::Menu {
                active_menu_idx,
                active_item_idx,
            } => {
                let items = crate::ui::menu::get_menu_items(active_menu_idx);
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Left => {
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
                    crossterm::event::KeyCode::Right => {
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
                    crossterm::event::KeyCode::Up => {
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
                    crossterm::event::KeyCode::Down => {
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
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
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
                    crossterm::event::KeyCode::Down => {
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
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
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
                    crossterm::event::KeyCode::Down => {
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
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::RenMovPrompt {
                            input: new_input,
                            src_paths: src_paths.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::RenMovPrompt {
                            input: new_input,
                            src_paths: src_paths.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
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
                                        state.active_popup = Some(PopupType::Error(format!(
                                            "Move failed: {}",
                                            e
                                        )));
                                        break;
                                    }
                                }
                            }
                        }

                        state.get_active_panel_mut().selected_paths.clear();
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::SearchPrompt {
                ref query,
                ref search_root,
            } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_query = query.clone();
                        new_query.push(c);
                        state.active_popup = Some(PopupType::SearchPrompt {
                            query: new_query,
                            search_root: search_root.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_query = query.clone();
                        new_query.pop();
                        state.active_popup = Some(PopupType::SearchPrompt {
                            query: new_query,
                            search_root: search_root.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let query = query.clone();
                        let search_root = search_root.clone();
                        if !query.is_empty() {
                            let results = search_files_recursive(&search_root, &query);
                            state.active_popup = Some(PopupType::SearchResults {
                                query,
                                results,
                                cursor_idx: 0,
                            });
                        } else {
                            state.active_popup = None;
                        }
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
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
                    crossterm::event::KeyCode::Down => {
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
                    crossterm::event::KeyCode::Enter => {
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
                if key.code == crossterm::event::KeyCode::Esc
                    || key.code == crossterm::event::KeyCode::Enter
                {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
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
                    crossterm::event::KeyCode::Down => {
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
                    crossterm::event::KeyCode::Enter => {
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
        }
    }
    Err(())
}

fn trigger_menu_item(
    state: &mut AppState,
    context: &mut AppContext,
    menu_idx: usize,
    item_idx: usize,
) -> Option<Action> {
    match menu_idx {
        0 => {
            // Left panel menu
            match item_idx {
                0 => {
                    // Brief View toggle
                    state.brief_view = !state.brief_view;
                    None
                }
                1 => {
                    // Full View: disable brief mode and refresh
                    state.brief_view = false;
                    state.refresh_both_panels(context.config.settings.show_hidden);
                    None
                }
                2 => {
                    // Info Panel
                    let lines = build_info_panel_lines(state);
                    state.active_popup = Some(PopupType::InfoPanel { lines });
                    None
                }
                3 => {
                    // Tree Panel
                    let root = state.left_panel.current_path.clone();
                    let nodes = build_tree_nodes(&root, 0, 2);
                    state.active_popup = Some(PopupType::TreeView {
                        nodes,
                        cursor_idx: 0,
                        panel: ActivePanel::Left,
                    });
                    None
                }
                4 => {
                    let drives = get_system_drives();
                    state.active_popup = Some(PopupType::DriveSelect {
                        panel: ActivePanel::Left,
                        drives,
                        cursor_idx: 0,
                    });
                    None
                }
                _ => None,
            }
        }
        1 => {
            // Files menu
            match item_idx {
                0 => Some(Action::Help),
                1 => Some(Action::UserMenu),
                2 => Some(Action::View),
                3 => Some(Action::Edit),
                4 => Some(Action::Copy),
                5 => Some(Action::Move),
                6 => Some(Action::MkDir),
                7 => Some(Action::Delete),
                8 => Some(Action::Quit),
                _ => None,
            }
        }
        2 => {
            // Commands menu
            match item_idx {
                0 => Some(Action::SwapPanels),
                1 => {
                    let same = compare_directories(state);
                    let msg = if same {
                        "Directory comparison: both directories contain identical file names."
                            .to_string()
                    } else {
                        "Directory comparison: directories differ in file names or structure."
                            .to_string()
                    };
                    // Use Info popup, not Error
                    state.active_popup = Some(PopupType::Info(msg));
                    None
                }
                2 => {
                    let bookmarks = get_hotlist_bookmarks();
                    state.active_popup = Some(PopupType::Hotlist {
                        bookmarks,
                        cursor_idx: 0,
                    });
                    None
                }
                3 => {
                    // Search Files
                    let search_root = state.get_active_panel().current_path.clone();
                    state.active_popup = Some(PopupType::SearchPrompt {
                        query: String::new(),
                        search_root,
                    });
                    None
                }
                _ => None,
            }
        }
        3 => {
            // Options menu
            match item_idx {
                0 => Some(Action::ToggleHidden),
                1 => {
                    change_theme(context, state, "slate");
                    None
                }
                2 => {
                    change_theme(context, state, "classic_blue");
                    None
                }
                3 => {
                    change_preset(context, "norton");
                    None
                }
                4 => {
                    change_preset(context, "vim");
                    None
                }
                _ => None,
            }
        }
        4 => {
            // Right panel menu
            match item_idx {
                0 => {
                    // Brief View toggle
                    state.brief_view = !state.brief_view;
                    None
                }
                1 => {
                    // Full View: disable brief mode and refresh
                    state.brief_view = false;
                    state.refresh_both_panels(context.config.settings.show_hidden);
                    None
                }
                2 => {
                    // Info Panel
                    let lines = build_info_panel_lines(state);
                    state.active_popup = Some(PopupType::InfoPanel { lines });
                    None
                }
                3 => {
                    // Tree Panel
                    let root = state.right_panel.current_path.clone();
                    let nodes = build_tree_nodes(&root, 0, 2);
                    state.active_popup = Some(PopupType::TreeView {
                        nodes,
                        cursor_idx: 0,
                        panel: ActivePanel::Right,
                    });
                    None
                }
                4 => {
                    let drives = get_system_drives();
                    state.active_popup = Some(PopupType::DriveSelect {
                        panel: ActivePanel::Right,
                        drives,
                        cursor_idx: 0,
                    });
                    None
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn get_system_drives() -> Vec<String> {
    let mut drives = Vec::new();
    if cfg!(target_os = "windows") {
        for drive_letter in b'A'..=b'Z' {
            let path = format!("{}:\\", drive_letter as char);
            if std::path::Path::new(&path).exists() {
                drives.push(path);
            }
        }
    } else {
        let paths = vec!["/", "/home", "/media", "/mnt", "/tmp"];
        for p in paths {
            if std::path::Path::new(p).exists() {
                drives.push(p.to_string());
            }
        }
    }
    if drives.is_empty() {
        drives.push("/".to_string());
    }
    drives
}

fn compare_directories(state: &AppState) -> bool {
    let left_names: std::collections::HashSet<String> = state
        .left_panel
        .entries
        .iter()
        .map(|e| e.name.clone())
        .collect();
    let right_names: std::collections::HashSet<String> = state
        .right_panel
        .entries
        .iter()
        .map(|e| e.name.clone())
        .collect();
    left_names == right_names
}

fn get_hotlist_bookmarks() -> Vec<(String, std::path::PathBuf)> {
    let mut bookmarks = Vec::new();
    if let Some(path) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) {
        bookmarks.push(("Home Directory".to_string(), path));
    }
    if let Some(path) =
        directories::UserDirs::new().and_then(|u| u.desktop_dir().map(|d| d.to_path_buf()))
    {
        bookmarks.push(("Desktop".to_string(), path));
    }
    if let Some(path) =
        directories::UserDirs::new().and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
    {
        bookmarks.push(("Documents".to_string(), path));
    }
    if let Some(path) =
        directories::UserDirs::new().and_then(|u| u.download_dir().map(|d| d.to_path_buf()))
    {
        bookmarks.push(("Downloads".to_string(), path));
    }
    bookmarks.push((
        "System Root".to_string(),
        std::path::PathBuf::from(if cfg!(target_os = "windows") {
            "C:\\"
        } else {
            "/"
        }),
    ));
    bookmarks
}

fn change_theme(context: &mut AppContext, state: &mut AppState, theme_name: &str) {
    context.config.settings.theme = theme_name.to_string();
    let theme = if theme_name == "classic_blue" {
        crate::config::theme::Theme::classic_blue()
    } else {
        crate::config::theme::Theme::default()
    };
    context.config.theme = theme;
    let _ = context.config.save();
    state.refresh_both_panels(context.config.settings.show_hidden);
}

fn change_preset(context: &mut AppContext, preset_name: &str) {
    context.config.keybindings.preset = preset_name.to_string();
    context.config.settings.keybinding_preset = preset_name.to_string();
    context.resolver = crate::keybindings::KeybindingResolver::new(&context.config);
    let _ = context.config.save();
}

/// Builds info panel lines for the currently highlighted entry.
fn build_info_panel_lines(state: &AppState) -> Vec<String> {
    let panel = state.get_active_panel();
    let mut lines = Vec::new();

    if let Some(entry) = panel.entries.get(panel.cursor_index) {
        lines.push(format!("Name    : {}", entry.name));
        lines.push(format!(
            "Type    : {}",
            if entry.is_dir { "Directory" } else { "File" }
        ));

        if !entry.is_dir {
            lines.push(format!("Size    : {} bytes", entry.size));
            if entry.size >= 1024 {
                lines.push(format!(
                    "        : {:.2} KB",
                    entry.size as f64 / 1024.0
                ));
            }
            if entry.size >= 1024 * 1024 {
                lines.push(format!(
                    "        : {:.2} MB",
                    entry.size as f64 / (1024.0 * 1024.0)
                ));
            }
        }

        if let Some(modified) = entry.modified {
            let datetime: chrono::DateTime<chrono::Local> = modified.into();
            lines.push(format!(
                "Modified: {}",
                datetime.format("%Y-%m-%d %H:%M:%S")
            ));
        }

        lines.push(String::new());
        lines.push(format!("Path    : {}", entry.path.to_string_lossy()));
    }

    lines.push(String::new());
    lines.push(format!(
        "Dir     : {}",
        panel.current_path.to_string_lossy()
    ));

    let total_files = panel.entries.iter().filter(|e| !e.is_dir).count();
    let total_dirs = panel.entries.iter().filter(|e| e.is_dir && e.name != "..").count();
    let total_size: u64 = panel.entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();

    lines.push(format!("Files   : {}", total_files));
    lines.push(format!("Folders : {}", total_dirs));
    lines.push(format!(
        "Total   : {:.2} MB",
        total_size as f64 / (1024.0 * 1024.0)
    ));
    lines.push(String::new());
    lines.push("[Enter/Esc] Close".to_string());
    lines
}

/// Recursively builds tree nodes up to `max_depth` levels deep.
fn build_tree_nodes(root: &std::path::Path, depth: usize, max_depth: usize) -> Vec<TreeNode> {
    let mut nodes = Vec::new();

    if depth == 0 {
        nodes.push(TreeNode {
            depth: 0,
            name: root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| root.to_string_lossy().to_string()),
            path: root.to_path_buf(),
            is_dir: true,
        });
    }

    if depth >= max_depth {
        return nodes;
    }

    if let Ok(read_dir) = std::fs::read_dir(root) {
        let mut entries: Vec<_> = read_dir.flatten().collect();
        entries.sort_by_key(|e| {
            let is_file = e.file_type().map(|ft| !ft.is_dir()).unwrap_or(false);
            (is_file, e.file_name())
        });

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files/system dirs
            if name.starts_with('.') {
                continue;
            }
            let is_dir = path.is_dir();

            nodes.push(TreeNode {
                depth: depth + 1,
                name: name.clone(),
                path: path.clone(),
                is_dir,
            });

            if is_dir && depth + 1 < max_depth {
                let children = build_tree_nodes(&path, depth + 1, max_depth);
                // Skip the root node of each recursive call (first element is the dir itself)
                nodes.extend(children.into_iter().skip(1));
            }
        }
    }

    nodes
}

/// Recursive file search — returns paths whose filenames contain `query` (case-insensitive).
fn search_files_recursive(root: &std::path::Path, query: &str) -> Vec<std::path::PathBuf> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();
    search_recursive_inner(root, &query_lower, &mut results, 0);
    results
}

fn search_recursive_inner(
    dir: &std::path::Path,
    query: &str,
    results: &mut Vec<std::path::PathBuf>,
    depth: usize,
) {
    // Limit recursion depth to keep search responsive
    if depth > 6 {
        return;
    }
    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.starts_with('.') {
                continue;
            }
            if name.contains(query) {
                results.push(path.clone());
            }
            if path.is_dir() {
                search_recursive_inner(&path, query, results, depth + 1);
            }
            // Cap results at 500 to avoid overwhelming the UI
            if results.len() >= 500 {
                return;
            }
        }
    }
}

/// Captures characters for bottom shell CLI command input.
fn handle_cli_input(
    state: &mut AppState,
    key: crossterm::event::KeyEvent,
    context: &AppContext,
    terminal_backend: &mut TerminalBackend,
) -> Result<(), ()> {
    if state.active_popup.is_some() {
        return Err(());
    }

    let is_vim = context.config.keybindings.preset == "vim";
    let is_active = !state.cli_input.is_empty() || !is_vim;

    if !is_active {
        return Err(());
    }

    match key.code {
        crossterm::event::KeyCode::Char(c) => {
            // Vim start trigger bypass
            if is_vim && state.cli_input.is_empty() && c == ':' {
                state.cli_input.push(' ');
                state.cli_input.clear();
                return Ok(());
            }

            if key.modifiers.is_empty() || key.modifiers == crossterm::event::KeyModifiers::SHIFT {
                state.cli_input.push(c);
                return Ok(());
            }
            Err(())
        }
        crossterm::event::KeyCode::Backspace => {
            if !state.cli_input.is_empty() {
                state.cli_input.pop();
                return Ok(());
            }
            Err(())
        }
        crossterm::event::KeyCode::Enter => {
            if !state.cli_input.is_empty() {
                let cmd = state.cli_input.trim().to_string();
                state.cli_input.clear();
                let _ = execute_shell_command(&cmd, terminal_backend);
                state.refresh_both_panels(context.config.settings.show_hidden);
                return Ok(());
            }
            Err(())
        }
        crossterm::event::KeyCode::Esc => {
            if !state.cli_input.is_empty() {
                state.cli_input.clear();
                return Ok(());
            }
            Err(())
        }
        _ => Err(()),
    }
}

/// Suspends raw mode **in-place**, runs a shell command natively, then re-enables raw mode.
/// Does NOT drop/recreate TerminalBackend to avoid double-restore.
fn execute_shell_command(command_str: &str, terminal_backend: &mut TerminalBackend) -> Result<()> {
    // Suspend TUI: leave alternate screen, disable raw mode
    terminal_backend.terminal.flush()?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)?;

    println!("\nNCRust shell execution: {}\n", command_str);

    let mut shell = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .arg("/c")
            .arg(command_str)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?
    };

    let _ = shell.wait();

    println!("\n[Press Enter to return to NCRust]");
    let mut buffer = String::new();
    let _ = std::io::stdin().read_line(&mut buffer);

    // Resume TUI: re-enable raw mode and re-enter alternate screen
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    terminal_backend.terminal.clear()?;
    Ok(())
}

/// Spawns an external utility (like a pager or editor) on the selected file path.
/// Suspends and resumes the TUI **in-place** without dropping/recreating TerminalBackend.
fn execute_external_command(
    target_path: &Path,
    utility_command: &str,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    // Suspend TUI
    terminal_backend.terminal.flush()?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)?;

    let mut child = std::process::Command::new(utility_command)
        .arg(target_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let _ = child.wait();

    println!("\n[Press Enter to return to NCRust]");
    let mut buffer = String::new();
    let _ = std::io::stdin().read_line(&mut buffer);

    // Resume TUI
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    terminal_backend.terminal.clear()?;
    Ok(())
}

/// Enters highlighted directory or open files with standard OS handlers.
fn handle_enter_key(state: &mut AppState, _show_hidden: bool) {
    let active = state.get_active_panel_mut();
    if let Some(entry) = active.entries.get(active.cursor_index) {
        if entry.is_dir {
            active.current_path = entry.path.clone();
            active.cursor_index = 0;
            active.selected_paths.clear();
        } else {
            let path = entry.path.to_string_lossy().to_string();
            let cmd = if cfg!(target_os = "windows") {
                format!("start \"\" \"{}\"", path)
            } else {
                format!("xdg-open \"{}\" 2>/dev/null", path)
            };

            let args = if cfg!(target_os = "windows") {
                vec!["/c", &cmd]
            } else {
                vec!["-c", &cmd]
            };

            let _ = std::process::Command::new(if cfg!(target_os = "windows") {
                "cmd"
            } else {
                "sh"
            })
            .args(&args)
            .spawn();
        }
    }
}

/// Ascends to parent folder directory.
fn handle_backspace_key(state: &mut AppState, show_hidden: bool) {
    let active = state.get_active_panel_mut();
    if let Some(parent) = active.current_path.parent() {
        let current_dir_name = active
            .current_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        active.current_path = parent.to_path_buf();
        active.selected_paths.clear();

        // Reread folder entries in parent directory
        state.refresh_both_panels(show_hidden);

        // Reposition cursor on directory we just exited
        let active_ref = match state.active_panel {
            ActivePanel::Left => &mut state.left_panel,
            ActivePanel::Right => &mut state.right_panel,
        };
        active_ref.cursor_index = active_ref
            .entries
            .iter()
            .position(|e| e.name == current_dir_name)
            .unwrap_or(0);
    }
}
