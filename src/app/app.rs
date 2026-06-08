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

    // Load history store from disk
    let history_store = crate::config::history::HistoryStore::load();
    state.command_history = history_store.commands.clone();
    state.file_view_history = history_store.viewed_files.clone();
    state.folders_history = history_store.visited_folders.clone();

    // Initial folder scans
    state.refresh_both_panels(context.config.settings.show_hidden);

    // Launch background external tools download/check
    tokio::spawn(async {
        if let Err(e) = crate::fs::external_tools::ensure_external_tools().await {
            log::warn!("Failed to download external tools: {}", e);
        }
    });

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
            // Save history store to disk
            let mut history_store = crate::config::history::HistoryStore::default();
            history_store.commands = state.command_history.clone();
            history_store.viewed_files = state.file_view_history.clone();
            history_store.visited_folders = state.folders_history.clone();
            let _ = history_store.save();
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
                Event::Resize(w, h) => {
                    log::debug!("Terminal resized to {}x{}", w, h);
                }
                Event::Tick => {}
                Event::Mouse(mouse) => {
                    log::debug!("Mouse event: {:?}", mouse);
                }
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
                let path = entry.path.clone();
                let entry_name = entry.name.clone();
                state.push_file_view_history(path.clone());

                let rule = crate::config::associations::AssociationsConfig::load()
                    .find_rule(&entry_name)
                    .cloned();

                if let Some(ref r) = rule {
                    let cmd = r.resolve_view_cmd(&path);
                    if let Err(e) = execute_external_command(&path, &cmd, terminal_backend) {
                        state.active_popup = Some(PopupType::Error(format!("Failed to run viewer: {}", e)));
                    }
                } else {
                    let viewer = crate::ui::viewer::ViewerState::load(path);
                    state.active_popup = Some(PopupType::InternalViewer { viewer });
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
                state.push_file_view_history(path.clone());
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
                            last_search: None,
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
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let dest_dir = state.get_passive_panel().current_path.clone();
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
        Action::CompressFiles => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let default_name = targets.first().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "archive".to_string());
                let dest_dir = state.get_passive_panel().current_path.clone();
                state.active_popup = Some(PopupType::CompressPrompt {
                    input: default_name,
                    targets,
                    dest_dir,
                });
            }
        }
        Action::ExtractArchive => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index).filter(|e| !e.is_dir) {
                let dest = state.get_passive_panel().current_path.clone();
                let rx = crate::fs::spawn_extract_task(entry.path.clone(), dest);
                state.progress_rx = Some(rx);
                state.active_popup = Some(PopupType::CopyProgress {
                    current_file: "Extracting...".to_string(),
                    files_copied: 0,
                    total_files: 0,
                    bytes_copied: 0,
                    total_bytes: 0,
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
        Action::ContextMenu => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let mut items = vec![
                    "1. View".to_string(),
                    "2. Edit".to_string(),
                    "3. Copy".to_string(),
                    "4. Move".to_string(),
                    "5. Delete".to_string(),
                    "6. Compress".to_string(),
                ];
                let has_archive = targets.iter().any(|p| {
                    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                    matches!(ext.as_str(), "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz")
                });
                if has_archive {
                    items.push("7. Extract".to_string());
                }
                state.active_popup = Some(PopupType::ContextMenu {
                    items,
                    cursor_idx: 0,
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
        Action::Refresh | Action::RereadPanel => {
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

        // ── Panel view modes ────────────────────────────────────────────────────
        Action::PanelViewBrief => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Brief; }
        Action::PanelViewMedium => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Medium; }
        Action::PanelViewFull => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Full; }
        Action::PanelViewWide => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Wide; }
        Action::PanelViewDetailed => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Detailed; }
        Action::PanelViewDescriptions => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Descriptions; }
        Action::PanelViewFileOwners => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::FileOwners; }
        Action::PanelViewFileLinks => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::FileLinks; }
        Action::PanelViewAltFull => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::AltFull; }

        // ── Panel visibility ────────────────────────────────────────────────────
        Action::TogglePanelLeft => { state.left_panel_visible = !state.left_panel_visible; }
        Action::TogglePanelRight => { state.right_panel_visible = !state.right_panel_visible; }
        Action::ToggleBothPanels => { state.both_panels_hidden = !state.both_panels_hidden; }
        Action::ToggleLongNames => {
            let panel = state.get_active_panel_mut();
            panel.show_long_names = !panel.show_long_names;
        }
        Action::InfoPanel => {
            let lines = build_info_panel_lines(state);
            state.active_popup = Some(PopupType::InfoPanel { lines });
        }
        Action::QuickView => {
            state.quick_view_active = !state.quick_view_active;
            if state.quick_view_active {
                let active = state.get_active_panel();
                if let Some(entry) = active.entries.get(active.cursor_index).filter(|e| !e.is_dir) {
                    let path = entry.path.clone();
                    let content = crate::ui::quickview::load_quick_view_content(&path);
                    state.active_popup = Some(PopupType::QuickViewPanel { path, content, scroll: 0 });
                }
            } else {
                if let Some(PopupType::QuickViewPanel { .. }) = state.active_popup {
                    state.active_popup = None;
                }
            }
        }
        Action::SortModes => {
            let current = state.get_active_panel().sort_field;
            let reverse = state.get_active_panel().sort_reverse;
            state.active_popup = Some(PopupType::SortModesDialog {
                current,
                reverse,
                cursor_idx: 0,
            });
        }

        // ── File operations ─────────────────────────────────────────────────────
        Action::WipeFile => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                state.active_popup = Some(PopupType::WipeConfirm { paths: targets });
            }
        }
        Action::CreateLink => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index) {
                if entry.name != ".." {
                    state.active_popup = Some(PopupType::CreateLinkPrompt {
                        src: entry.path.clone(),
                        dest_input: entry.name.clone(),
                        kind: crate::app::state::LinkKind::Symbolic,
                    });
                }
            }
        }
        Action::FileAttributes => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index) {
                if entry.name != ".." {
                    match crate::fs::read_attrs(&entry.path) {
                        Ok(attrs) => {
                            let mode_octal = format!("{:o}", attrs.mode & 0o7777);
                            state.active_popup = Some(PopupType::FileAttributesDialog {
                                attrs: crate::app::state::FileAttrsSnapshot {
                                    path: attrs.path,
                                    readonly: attrs.readonly,
                                    size: attrs.size,
                                    modified: attrs.modified,
                                    created: attrs.created,
                                    owner: attrs.owner,
                                    nlinks: attrs.nlinks,
                                },
                                mode_input: mode_octal,
                            });
                        }
                        Err(e) => {
                            state.active_popup = Some(PopupType::Error(format!("Cannot read attrs: {}", e)));
                        }
                    }
                }
            }
        }
        Action::ApplyCommand => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                state.active_popup = Some(PopupType::ApplyCommandPrompt {
                    input: String::new(),
                    targets,
                });
            }
        }
        Action::DescribeFile => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index) {
                if entry.name != ".." {
                    let current_desc = crate::fs::read_description(
                        &active.current_path.clone(),
                        &entry.name,
                    )
                    .unwrap_or_default();
                    state.active_popup = Some(PopupType::DescribeFilePrompt {
                        path: entry.path.clone(),
                        current_desc: current_desc.clone(),
                        input: current_desc,
                    });
                }
            }
        }
        Action::ArchiveCommands => {
            let active = state.get_active_panel();
            if let Some(entry) = active.entries.get(active.cursor_index).filter(|e| !e.is_dir) {
                state.active_popup = Some(PopupType::ArchiveCommandsMenu {
                    archive_path: entry.path.clone(),
                    items: vec![
                        "1. List contents".to_string(),
                        "2. Test integrity".to_string(),
                        "3. Extract here".to_string(),
                        "4. Extract to other panel".to_string(),
                    ],
                    cursor_idx: 0,
                });
            }
        }

        // ── Bulk selection ──────────────────────────────────────────────────────
        Action::SelectGroup => {
            state.active_popup = Some(PopupType::SelectGroupPrompt {
                mode: crate::app::state::SelectMode::Add,
                query: String::new(),
            });
        }
        Action::UnselectGroup => {
            state.active_popup = Some(PopupType::SelectGroupPrompt {
                mode: crate::app::state::SelectMode::Remove,
                query: String::new(),
            });
        }
        Action::InvertSelection => {
            state.snapshot_selection();
            state.get_active_panel_mut().invert_selection();
        }
        Action::RestoreSelection => {
            state.restore_selection();
        }

        // ── Search & history ────────────────────────────────────────────────────
        Action::FindFile => {
            let root = state.get_active_panel().current_path.clone();
            state.active_popup = Some(PopupType::SearchPrompt {
                query: String::new(),
                content_query: String::new(),
                search_root: root,
                focus_content: false,
            });
        }
        Action::TreeView => {
            let root = state.get_active_panel().current_path.clone();
            let nodes = build_tree_nodes(&root, 0, 3);
            state.active_popup = Some(PopupType::TreeView {
                nodes,
                cursor_idx: 0,
                panel: state.active_panel,
            });
        }
        Action::CommandHistory => {
            let entries = state.command_history.clone();
            state.active_popup = Some(PopupType::CommandHistoryList { entries, cursor_idx: 0 });
        }
        Action::FileViewHistory => {
            let entries = state.file_view_history.clone();
            state.active_popup = Some(PopupType::FileViewHistoryList { entries, cursor_idx: 0 });
        }
        Action::FoldersHistory => {
            let entries = state.folders_history.clone();
            state.active_popup = Some(PopupType::FoldersHistoryList { entries, cursor_idx: 0 });
        }

        // ── Commands ────────────────────────────────────────────────────────────
        Action::CompareFolder => {
            let left = state.left_panel.current_path.clone();
            let right = state.right_panel.current_path.clone();
            match crate::fs::compare_directories(&left, &right) {
                Ok(diff) => {
                    for entry in &diff {
                        if entry.status != crate::fs::CompareStatus::Equal {
                            if let Some(e) = state.left_panel.entries.iter().find(|e| e.name == entry.name) {
                                state.left_panel.selected_paths.insert(e.path.clone());
                            }
                        }
                    }
                    state.active_popup = Some(PopupType::CompareFoldersResult { diff, cursor_idx: 0 });
                }
                Err(e) => {
                    state.active_popup = Some(PopupType::Error(format!("Compare failed: {}", e)));
                }
            }
        }
        Action::EditUserMenu => {
            state.active_popup = Some(PopupType::Info(
                "Edit user menu: open UserMenu config file with default editor.".to_string(),
            ));
        }
        Action::FileAssociations => {
            let config = crate::config::associations::AssociationsConfig::load();
            state.active_popup = Some(PopupType::FileAssociationsDialog {
                rules: config.rules,
                cursor_idx: 0,
            });
        }
        Action::FolderShortcutsConfig => {
            let bookmarks = get_hotlist_bookmarks();
            state.active_popup = Some(PopupType::Hotlist {
                bookmarks,
                cursor_idx: 0,
            });
        }
        Action::FilePanelFilter => {
            let current = state.get_active_panel().filter_mask.clone().unwrap_or_default();
            state.active_popup = Some(PopupType::FilePanelFilterPrompt { input: current });
        }
        Action::TaskList => {
            let tasks = get_process_list();
            state.active_popup = Some(PopupType::TaskListDialog { tasks, cursor_idx: 0 });
        }

        Action::SaveSetup => {
            state.active_popup = Some(PopupType::SaveSetupConfirm);
        }
        Action::SystemSettings => {
            state.active_popup = Some(PopupType::Info(
                "System settings: edit ~/.config/ncrust/config.toml directly.".to_string(),
            ));
        }

        // ── Folder shortcuts navigation ──────────────────────────────────────────
        Action::GoFolderShortcut(n) => {
            if let Some(target) = state.folder_shortcuts.get(&n).cloned() {
                let panel = state.get_active_panel_mut();
                panel.current_path = target;
                panel.cursor_index = 0;
                panel.selected_paths.clear();
                state.refresh_both_panels(context.config.settings.show_hidden);
            } else {
                state.active_popup = Some(PopupType::Info(
                    format!("No folder shortcut assigned to Ctrl+Alt+{}", n),
                ));
            }
        }

        // ── Stubs ─────────────────────────────────────────────────────────────
        Action::PluginMenu => {
            state.active_popup = Some(PopupType::Info("Plugin system: not yet implemented.".to_string()));
        }
        Action::ScreensList => {
            state.active_popup = Some(PopupType::Info("Screens list: not yet implemented.".to_string()));
        }
        Action::VideoMode => {
            state.active_popup = Some(PopupType::Info("Video mode: resize your terminal manually.".to_string()));
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
                    crossterm::event::KeyCode::Char('6') => {
                        state.active_popup = None;
                        let (tx, rx) = tokio::sync::mpsc::channel(100);
                        tokio::spawn(async move {
                            let _ = tx.send(crate::fs::ProgressUpdate {
                                current_file: "Downloading 7z...".to_string(),
                                files_copied: 0,
                                total_files: 1,
                                bytes_copied: 0,
                                total_bytes: 1,
                                error: None,
                            }).await;

                            if let Err(e) = crate::fs::external_tools::ensure_external_tools().await {
                                let _ = tx.send(crate::fs::ProgressUpdate {
                                    current_file: "Completed".to_string(),
                                    files_copied: 0, total_files: 1, bytes_copied: 0, total_bytes: 1,
                                    error: Some(format!("Failed to download: {}", e)),
                                }).await;
                            } else {
                                let _ = tx.send(crate::fs::ProgressUpdate {
                                    current_file: "Completed".to_string(),
                                    files_copied: 1, total_files: 1, bytes_copied: 1, total_bytes: 1,
                                    error: None,
                                }).await;
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

                let is_ctrl = key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);
                let is_shift = key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT);

                match key.code {
                    crossterm::event::KeyCode::Char(c) if !is_ctrl => {
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
                            if cursor_y >= scroll_y + edit_height {
                                scroll_y = cursor_y.saturating_sub(edit_height - 1);
                            }
                        }
                    }
                    crossterm::event::KeyCode::PageUp => {
                        cursor_y = cursor_y.saturating_sub(edit_height);
                        cursor_x = cursor_x.min(lines[cursor_y].len());
                        if cursor_y < scroll_y {
                            scroll_y = cursor_y;
                        }
                    }
                    crossterm::event::KeyCode::PageDown => {
                        cursor_y = (cursor_y + edit_height).min(lines.len().saturating_sub(1));
                        cursor_x = cursor_x.min(lines[cursor_y].len());
                        if cursor_y >= scroll_y + edit_height {
                            scroll_y = cursor_y.saturating_sub(edit_height - 1);
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
                    crossterm::event::KeyCode::Char('s') if is_ctrl => {
                        let content = lines.join("\n");
                        if let Err(e) = std::fs::write(&path, content) {
                            state.active_popup =
                                Some(PopupType::Error(format!("Failed to save: {}", e)));
                            return Ok(None);
                        }
                        is_dirty = false;
                    }
                    crossterm::event::KeyCode::Char('r') if is_ctrl => {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let reloaded_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                                lines = if reloaded_lines.is_empty() {
                                    vec![String::new()]
                                } else {
                                    reloaded_lines
                                };
                                cursor_x = cursor_x.min(lines.get(cursor_y).map(|l| l.len()).unwrap_or(0));
                                is_dirty = false;
                            }
                            Err(e) => {
                                state.active_popup = Some(PopupType::Error(format!("Failed to reload: {}", e)));
                                return Ok(None);
                            }
                        }
                    }
                    crossterm::event::KeyCode::Char('d') if is_ctrl => {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let reloaded_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                                lines = if reloaded_lines.is_empty() {
                                    vec![String::new()]
                                } else {
                                    reloaded_lines
                                };
                                cursor_x = cursor_x.min(lines.get(cursor_y).map(|l| l.len()).unwrap_or(0));
                                is_dirty = false;
                            }
                            Err(e) => {
                                state.active_popup = Some(PopupType::Error(format!("Failed to reload: {}", e)));
                                return Ok(None);
                            }
                        }
                    }
                    crossterm::event::KeyCode::F(7) if is_shift => {
                        if let Some(ref q) = last_search {
                            if let Some((found_x, found_y)) = find_next_in_editor(&lines, cursor_x, cursor_y, q) {
                                cursor_x = found_x;
                                cursor_y = found_y;
                                if cursor_y < scroll_y || cursor_y >= scroll_y + edit_height {
                                    scroll_y = cursor_y.saturating_sub(edit_height / 2);
                                }
                            }
                        }
                    }
                    crossterm::event::KeyCode::F(7) => {
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
                    crossterm::event::KeyCode::Char('f') if is_ctrl => {
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
                    crossterm::event::KeyCode::F(3) => {
                        if let Some(ref q) = last_search {
                            if let Some((found_x, found_y)) = find_next_in_editor(&lines, cursor_x, cursor_y, q) {
                                cursor_x = found_x;
                                cursor_y = found_y;
                                if cursor_y < scroll_y || cursor_y >= scroll_y + edit_height {
                                    scroll_y = cursor_y.saturating_sub(edit_height / 2);
                                }
                            }
                        }
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

                let is_ctrl = key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);

                match key.code {
                    crossterm::event::KeyCode::Char(c) if !is_ctrl => {
                        query.push(c);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        query.pop();
                    }
                    crossterm::event::KeyCode::Esc => {
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
                    crossterm::event::KeyCode::Enter => {
                        let q = query.clone();
                        if !q.is_empty() {
                            if let Some((found_x, found_y)) = find_next_in_editor(&lines, cursor_x, cursor_y, &q) {
                                let new_cursor_x = found_x;
                                let new_cursor_y = found_y;
                                let mut new_scroll_y = scroll_y;
                                if new_cursor_y < new_scroll_y || new_cursor_y >= new_scroll_y + edit_height {
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
                                state.active_popup = Some(PopupType::Error("Text not found".to_string()));
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
                    crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::F(10) => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
                        viewer.scroll_up(1);
                    }
                    crossterm::event::KeyCode::Down => {
                        viewer.scroll_down(1);
                    }
                    crossterm::event::KeyCode::PageUp => {
                        viewer.scroll_up(18);
                    }
                    crossterm::event::KeyCode::PageDown => {
                        viewer.scroll_down(18);
                    }
                    crossterm::event::KeyCode::F(2) => {
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
                ref content_query,
                ref search_root,
                focus_content,
            } => {
                match key.code {
                    crossterm::event::KeyCode::Tab | crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Down => {
                        state.active_popup = Some(PopupType::SearchPrompt {
                            query: query.clone(),
                            content_query: content_query.clone(),
                            search_root: search_root.clone(),
                            focus_content: !focus_content,
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char(c) => {
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
                    crossterm::event::KeyCode::Backspace => {
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
                    crossterm::event::KeyCode::Enter => {
                        let q = query.clone();
                        let c_q = content_query.clone();
                        let search_root = search_root.clone();
                        if !q.is_empty() || !c_q.is_empty() {
                            let results = search_files_recursive(&search_root, &q, if c_q.is_empty() { None } else { Some(&c_q) });
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
            PopupType::ContextMenu {
                ref items,
                cursor_idx,
            } => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
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
                    crossterm::event::KeyCode::Down => {
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
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::CompressPrompt {
                            input: new_input,
                            targets: targets.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::CompressPrompt {
                            input: new_input,
                            targets: targets.clone(),
                            dest_dir: dest_dir.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::WipeConfirm { ref paths } => {
                match key.code {
                    crossterm::event::KeyCode::Enter => {
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
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Err(());
            }
            PopupType::SelectGroupPrompt { ref mode, ref query } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_q = query.clone();
                        new_q.push(c);
                        state.active_popup = Some(PopupType::SelectGroupPrompt { mode: mode.clone(), query: new_q });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_q = query.clone();
                        new_q.pop();
                        state.active_popup = Some(PopupType::SelectGroupPrompt { mode: mode.clone(), query: new_q });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let mode = mode.clone();
                        let query = query.clone();
                        state.active_popup = None;
                        match mode {
                            crate::app::state::SelectMode::Add => state.get_active_panel_mut().select_group(&query),
                            crate::app::state::SelectMode::Remove => state.get_active_panel_mut().unselect_group(&query),
                        }
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => { state.active_popup = None; return Ok(None); }
                    _ => {}
                }
                return Err(());
            }
            PopupType::ApplyCommandPrompt { ref input, ref targets } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::ApplyCommandPrompt { input: new_input, targets: targets.clone() });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::ApplyCommandPrompt { input: new_input, targets: targets.clone() });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let cmd = input.clone();
                        let targets = targets.clone();
                        state.active_popup = None;
                        if !cmd.is_empty() {
                            let rx = crate::fs::apply_command(cmd, targets);
                            state.progress_rx = Some(rx);
                            state.active_popup = Some(PopupType::CopyProgress {
                                current_file: "Running command...".to_string(),
                                files_copied: 0, total_files: 0, bytes_copied: 0, total_bytes: 0,
                            });
                        }
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => { state.active_popup = None; return Ok(None); }
                    _ => {}
                }
                return Err(());
            }
            PopupType::DescribeFilePrompt { ref path, ref current_desc, ref input } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::DescribeFilePrompt {
                            path: path.clone(), current_desc: current_desc.clone(), input: new_input,
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::DescribeFilePrompt {
                            path: path.clone(), current_desc: current_desc.clone(), input: new_input,
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let desc = input.clone();
                        let p = path.clone();
                        state.active_popup = None;
                        if let Some(dir) = p.parent() {
                            if let Some(name) = p.file_name() {
                                let _ = crate::fs::write_description(dir, &name.to_string_lossy(), &desc);
                            }
                        }
                        state.refresh_both_panels(context.config.settings.show_hidden);
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => { state.active_popup = None; return Ok(None); }
                    _ => {}
                }
                return Err(());
            }
            PopupType::CreateLinkPrompt { ref src, ref dest_input, ref kind } => {
                match key.code {
                    crossterm::event::KeyCode::Char('s') | crossterm::event::KeyCode::Char('h') => {
                        let new_kind = match key.code {
                            crossterm::event::KeyCode::Char('s') => crate::app::state::LinkKind::Symbolic,
                            _ => crate::app::state::LinkKind::Hard,
                        };
                        state.active_popup = Some(PopupType::CreateLinkPrompt {
                            src: src.clone(), dest_input: dest_input.clone(), kind: new_kind,
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char(c) if !matches!(c, 's' | 'h') => {
                        let mut new_input = dest_input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::CreateLinkPrompt {
                            src: src.clone(), dest_input: new_input, kind: kind.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = dest_input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::CreateLinkPrompt {
                            src: src.clone(), dest_input: new_input, kind: kind.clone(),
                        });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let s = src.clone();
                        let kind = kind.clone();
                        let dest = state.get_passive_panel().current_path.join(dest_input);
                        state.active_popup = None;
                        let result = match kind {
                            crate::app::state::LinkKind::Symbolic => crate::fs::create_symlink(&s, &dest),
                            crate::app::state::LinkKind::Hard => crate::fs::create_hardlink(&s, &dest),
                        };
                        if let Err(e) = result {
                            state.active_popup = Some(PopupType::Error(format!("Link failed: {}", e)));
                        } else {
                            state.refresh_both_panels(context.config.settings.show_hidden);
                        }
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => { state.active_popup = None; return Ok(None); }
                    _ => {}
                }
                return Err(());
            }
            PopupType::FilePanelFilterPrompt { ref input } => {
                match key.code {
                    crossterm::event::KeyCode::Char(c) => {
                        let mut new_input = input.clone();
                        new_input.push(c);
                        state.active_popup = Some(PopupType::FilePanelFilterPrompt { input: new_input });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        let mut new_input = input.clone();
                        new_input.pop();
                        state.active_popup = Some(PopupType::FilePanelFilterPrompt { input: new_input });
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Enter => {
                        let mask = input.trim().to_string();
                        state.active_popup = None;
                        let panel = state.get_active_panel_mut();
                        panel.filter_mask = if mask.is_empty() { None } else { Some(mask) };
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => { state.active_popup = None; return Ok(None); }
                    _ => {}
                }
                return Err(());
            }
            PopupType::TaskListDialog { mut tasks, mut cursor_idx } => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Up => {
                        if cursor_idx > 0 {
                            cursor_idx -= 1;
                        }
                    }
                    crossterm::event::KeyCode::Down => {
                        if !tasks.is_empty() && cursor_idx < tasks.len().saturating_sub(1) {
                            cursor_idx += 1;
                        }
                    }
                    crossterm::event::KeyCode::Delete | crossterm::event::KeyCode::Char('k') => {
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
                                    state.active_popup = Some(PopupType::Error(format!("Failed to kill process: {}", e)));
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
                if key.code == crossterm::event::KeyCode::Esc
                    || key.code == crossterm::event::KeyCode::Enter
                {
                    state.active_popup = None;
                    return Ok(None);
                }
                return Err(());
            }
            PopupType::SaveSetupConfirm => {
                match key.code {
                    crossterm::event::KeyCode::Enter => {
                        match context.config.save() {
                            Ok(_) => {
                                state.active_popup = Some(PopupType::Info("Configuration saved successfully.".to_string()));
                            }
                            Err(e) => {
                                state.active_popup = Some(PopupType::Error(format!("Failed to save setup: {}", e)));
                            }
                        }
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    _ => {}
                }
                return Ok(None);
            }
            PopupType::FileAttributesDialog { mut attrs, mut mode_input } => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        state.active_popup = None;
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char(c) if c.is_digit(8) => {
                        if mode_input.len() < 4 {
                            mode_input.push(c);
                        }
                    }
                    crossterm::event::KeyCode::Backspace => {
                        mode_input.pop();
                    }
                    crossterm::event::KeyCode::Char('r') | crossterm::event::KeyCode::Char('R') | crossterm::event::KeyCode::Char(' ') => {
                        attrs.readonly = !attrs.readonly;
                    }
                    crossterm::event::KeyCode::Enter => {
                        if !mode_input.is_empty() {
                            if let Ok(mode) = u32::from_str_radix(&mode_input, 8) {
                                if let Err(e) = crate::fs::attrs::set_unix_mode(&attrs.path, mode) {
                                    state.active_popup = Some(PopupType::Error(format!("Failed to set unix mode: {}", e)));
                                    return Ok(None);
                                }
                            }
                        }
                        if let Err(e) = crate::fs::attrs::set_readonly(&attrs.path, attrs.readonly) {
                            state.active_popup = Some(PopupType::Error(format!("Failed to set readonly: {}", e)));
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

fn kill_process(pid: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("kill failed: {}", err_msg),
            ))
        }
    }
    #[cfg(not(unix))]
    {
        let output = std::process::Command::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .arg(pid.to_string())
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("taskkill failed: {}", err_msg),
            ))
        }
    }
}

fn trigger_menu_item(
    state: &mut AppState,
    context: &mut AppContext,
    menu_idx: usize,
    item_idx: usize,
) -> Option<Action> {
    // Skip separator lines (those starting with " ─") when mapping item_idx to action.
    // The index is the raw cursor position in the menu; separator lines are not actionable.
    match menu_idx {
        0 | 4 => {
            // Left / Right panel menu — both have the same layout
            let is_right = menu_idx == 4;
            match item_idx {
                0 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Brief; None }
                1 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Medium; None }
                2 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Full; None }
                3 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Wide; None }
                4 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Detailed; None }
                5 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Descriptions; None }
                6 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::FileOwners; None }
                7 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::FileLinks; None }
                8 => { state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::AltFull; None }
                // 9 = separator
                10 => Some(Action::InfoPanel),
                11 => Some(Action::QuickView),
                // 12 = separator
                13 => Some(Action::SortModes),
                14 => Some(Action::ToggleLongNames),
                15 => {
                    if is_right { Some(Action::TogglePanelRight) } else { Some(Action::TogglePanelLeft) }
                }
                16 => Some(Action::Refresh),
                17 => {
                    if is_right { Some(Action::DriveSelectRight) } else { Some(Action::DriveSelectLeft) }
                }
                _ => None,
            }
        }
        1 => {
            // Files menu
            match item_idx {
                0 => Some(Action::View),
                1 => Some(Action::Edit),
                2 => Some(Action::Copy),
                3 => Some(Action::Move),
                4 => Some(Action::CreateLink),
                5 => Some(Action::MkDir),
                6 => Some(Action::Delete),
                7 => Some(Action::WipeFile),
                // 8 = separator
                9 => Some(Action::CompressFiles),
                10 => Some(Action::ExtractArchive),
                11 => Some(Action::ArchiveCommands),
                // 12 = separator
                13 => Some(Action::FileAttributes),
                14 => Some(Action::ApplyCommand),
                15 => Some(Action::DescribeFile),
                // 16 = separator
                17 => Some(Action::SelectGroup),
                18 => Some(Action::UnselectGroup),
                19 => Some(Action::InvertSelection),
                20 => Some(Action::RestoreSelection),
                _ => None,
            }
        }
        2 => {
            // Commands menu
            match item_idx {
                0 => Some(Action::FindFile),
                1 => Some(Action::CommandHistory),
                2 => Some(Action::FileViewHistory),
                3 => Some(Action::FoldersHistory),
                // 4 = separator
                5 => Some(Action::SwapPanels),
                6 => Some(Action::ToggleBothPanels),
                7 => Some(Action::CompareFolder),
                // 8 = separator
                9 => Some(Action::EditUserMenu),
                10 => Some(Action::FileAssociations),
                11 => Some(Action::FolderShortcutsConfig),
                12 => Some(Action::FilePanelFilter),
                // 13 = separator
                14 => Some(Action::PluginMenu),
                15 => Some(Action::ScreensList),
                16 => Some(Action::TaskList),
                17 => {
                    // Hotplug devices — reuse DriveSelect
                    let drives = get_system_drives();
                    state.active_popup = Some(PopupType::DriveSelect {
                        panel: state.active_panel,
                        drives,
                        cursor_idx: 0,
                    });
                    None
                }
                _ => None,
            }
        }
        3 => {
            // Options menu
            match item_idx {
                0 | 1 | 2 => {
                    // System/Panel/Interface settings — show info stub
                    state.active_popup = Some(PopupType::Info(
                        "Edit settings file: ~/.config/ncrust/config.toml".to_string(),
                    ));
                    None
                }
                // 3 = separator
                4 | 5 | 6 => {
                    state.active_popup = Some(PopupType::Info(
                        "File descriptions: use Ctrl+Z on files.".to_string(),
                    ));
                    None
                }
                // 7 = separator
                8 | 9 | 10 => {
                    state.active_popup = Some(PopupType::Info(
                        "Viewer/Editor settings: edit config.toml [viewer] / [editor] sections.".to_string(),
                    ));
                    None
                }
                // 11 = separator
                12 | 13 => {
                    state.active_popup = Some(PopupType::Info(
                        "Colors: edit the [theme] section in config.toml.".to_string(),
                    ));
                    None
                }
                // 14 = separator
                15 => { change_theme(context, state, "slate"); None }
                16 => { change_theme(context, state, "classic_blue"); None }
                // 17 = separator
                18 => { change_preset(context, "norton"); None }
                19 => { change_preset(context, "vim"); None }
                20 => { change_preset(context, "modern"); None }
                // 21 = separator
                22 => Some(Action::ToggleHidden),
                23 => Some(Action::SaveSetup),
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

/// Returns a list of running OS processes.
/// On Linux reads from /proc; on other platforms returns an empty list.
fn get_process_list() -> Vec<crate::app::state::ProcessEntry> {
    let mut processes = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Ok(read_dir) = std::fs::read_dir("/proc") {
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // /proc/<pid> directories have purely numeric names
                if let Ok(pid) = name_str.parse::<u32>() {
                    let comm_path = entry.path().join("comm");
                    let proc_name = std::fs::read_to_string(&comm_path)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    // Read VmRSS from status for memory approximation
                    let memory_kb = read_proc_memory(pid);
                    processes.push(crate::app::state::ProcessEntry {
                        pid,
                        name: if proc_name.is_empty() { format!("[{}]", pid) } else { proc_name },
                        memory_kb,
                    });
                }
            }
        }
        processes.sort_by_key(|p| p.pid);
    }

    processes
}

#[cfg(target_os = "linux")]
fn read_proc_memory(pid: u32) -> u64 {
    let status_path = format!("/proc/{}/status", pid);
    if let Ok(content) = std::fs::read_to_string(&status_path) {
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb_str) = parts.get(1) {
                    return kb_str.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }
    0
}

// This bookmarks resolution logic is prepared for the hotlist/quick bookmarks panel feature.
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

// Recursively builds tree nodes for the graphical tree navigator feature.
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

/// Recursive file search — returns paths whose filenames contain `query` (case-insensitive)
/// and optionally contain the requested search content.
fn search_files_recursive(
    root: &std::path::Path,
    query: &str,
    content_query: Option<&str>,
) -> Vec<std::path::PathBuf> {
    let name_glob = if query.is_empty() {
        "".to_string()
    } else if query.contains('*') || query.contains('?') {
        query.to_string()
    } else {
        format!("*{}*", query)
    };

    let q = crate::fs::search::SearchQuery {
        name_glob,
        content: content_query.map(|s| s.to_string()),
        root: root.to_path_buf(),
    };

    let mut rx = crate::fs::search::find_files(q);
    let mut results = Vec::new();
    while let Some(path) = rx.blocking_recv() {
        results.push(path);
        if results.len() >= 500 {
            break;
        }
    }
    results
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
                state.push_command_history(cmd.clone());
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

// Suspends TUI and launches an external editor or viewer command (reserved for custom user command association bindings).
fn execute_external_command(
    _target_path: &Path,
    utility_command: &str,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    // Suspend TUI
    terminal_backend.terminal.flush()?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)?;

    let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
    let flag = if cfg!(target_os = "windows") { "/c" } else { "-c" };
    let mut child = std::process::Command::new(shell)
        .arg(flag)
        .arg(utility_command)
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
    let mut target_dir = None;
    {
        let active = state.get_active_panel();
        if let Some(entry) = active.entries.get(active.cursor_index) {
            if entry.is_dir {
                target_dir = Some(entry.path.clone());
            } else {
                let path = entry.path.to_string_lossy().to_string();
                let rule = crate::config::associations::AssociationsConfig::load()
                    .find_rule(&entry.name)
                    .cloned();

                let cmd = if let Some(r) = rule {
                    r.resolve_open_cmd(&entry.path)
                } else if cfg!(target_os = "windows") {
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
    if let Some(dir) = target_dir {
        state.push_folders_history(dir.clone());
        let active_mut = state.get_active_panel_mut();
        active_mut.current_path = dir;
        active_mut.cursor_index = 0;
        active_mut.selected_paths.clear();
    }
}

/// Ascends to parent folder directory.
fn handle_backspace_key(state: &mut AppState, show_hidden: bool) {
    let parent_path = state.get_active_panel().current_path.parent().map(|p| p.to_path_buf());
    if let Some(parent) = parent_path {
        let current_dir_name = state.get_active_panel()
            .current_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        state.push_folders_history(parent.clone());

        state.get_active_panel_mut().current_path = parent;
        state.get_active_panel_mut().selected_paths.clear();

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

fn find_next_in_editor(lines: &[String], current_x: usize, current_y: usize, query: &str) -> Option<(usize, usize)> {
    if query.is_empty() || lines.is_empty() {
        return None;
    }
    let q_lower = query.to_lowercase();
    
    // 1. Search current line forward (starting at current_x + 1)
    if current_y < lines.len() {
        let line = &lines[current_y];
        let start_idx = current_x + 1;
        if start_idx < line.len() {
            if let Some(pos) = line[start_idx..].to_lowercase().find(&q_lower) {
                return Some((start_idx + pos, current_y));
            }
        }
    }

    // 2. Search subsequent lines forward
    for y in (current_y + 1)..lines.len() {
        if let Some(pos) = lines[y].to_lowercase().find(&q_lower) {
            return Some((pos, y));
        }
    }

    // 3. Wrap around: Search from start of file up to current_y
    for y in 0..=current_y {
        let line = &lines[y];
        let limit = if y == current_y { current_x } else { line.len() };
        if let Some(pos) = line[..limit].to_lowercase().find(&q_lower) {
            return Some((pos, y));
        }
    }
    
    None
}
