use super::input::{handle_backspace_key, handle_enter_key};
use super::sys_helpers::{
    build_info_panel_lines, build_tree_nodes, get_hotlist_bookmarks, get_process_list,
    get_system_drives,
};
use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::keybindings::Action;
use crate::terminal::TerminalBackend;
use anyhow::Result;
use crossterm::{
    cursor::Show,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::path::Path;

/// Dispatches actions to their respective state changes.
pub async fn handle_action(
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
                        state.active_popup =
                            Some(PopupType::Error(format!("Failed to run viewer: {}", e)));
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
                let default_name = targets
                    .first()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "archive".to_string());
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
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
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
                    let ext = p
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    matches!(
                        ext.as_str(),
                        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz"
                    )
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
        Action::PanelViewBrief => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Brief;
        }
        Action::PanelViewMedium => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Medium;
        }
        Action::PanelViewFull => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Full;
        }
        Action::PanelViewWide => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Wide;
        }
        Action::PanelViewDetailed => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Detailed;
        }
        Action::PanelViewDescriptions => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::Descriptions;
        }
        Action::PanelViewFileOwners => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::FileOwners;
        }
        Action::PanelViewFileLinks => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::FileLinks;
        }
        Action::PanelViewAltFull => {
            state.get_active_panel_mut().view_mode = crate::app::state::PanelViewMode::AltFull;
        }

        // ── Panel visibility ────────────────────────────────────────────────────
        Action::TogglePanelLeft => {
            state.left_panel_visible = !state.left_panel_visible;
        }
        Action::TogglePanelRight => {
            state.right_panel_visible = !state.right_panel_visible;
        }
        Action::ToggleBothPanels => {
            state.both_panels_hidden = !state.both_panels_hidden;
        }
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
                if let Some(entry) = active
                    .entries
                    .get(active.cursor_index)
                    .filter(|e| !e.is_dir)
                {
                    let path = entry.path.clone();
                    let content = crate::ui::quickview::load_quick_view_content(&path);
                    state.active_popup = Some(PopupType::QuickViewPanel {
                        path,
                        content,
                        scroll: 0,
                    });
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
                            state.active_popup =
                                Some(PopupType::Error(format!("Cannot read attrs: {}", e)));
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
                    let current_desc =
                        crate::fs::read_description(&active.current_path.clone(), &entry.name)
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
            if let Some(entry) = active
                .entries
                .get(active.cursor_index)
                .filter(|e| !e.is_dir)
            {
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
            state.active_popup = Some(PopupType::CommandHistoryList {
                entries,
                cursor_idx: 0,
            });
        }
        Action::FileViewHistory => {
            let entries = state.file_view_history.clone();
            state.active_popup = Some(PopupType::FileViewHistoryList {
                entries,
                cursor_idx: 0,
            });
        }
        Action::FoldersHistory => {
            let entries = state.folders_history.clone();
            state.active_popup = Some(PopupType::FoldersHistoryList {
                entries,
                cursor_idx: 0,
            });
        }

        // ── Commands ────────────────────────────────────────────────────────────
        Action::CompareFolder => {
            let left = state.left_panel.current_path.clone();
            let right = state.right_panel.current_path.clone();
            match crate::fs::compare_directories(&left, &right) {
                Ok(diff) => {
                    for entry in &diff {
                        if entry.status != crate::fs::CompareStatus::Equal {
                            if let Some(e) = state
                                .left_panel
                                .entries
                                .iter()
                                .find(|e| e.name == entry.name)
                            {
                                state.left_panel.selected_paths.insert(e.path.clone());
                            }
                        }
                    }
                    state.active_popup = Some(PopupType::CompareFoldersResult {
                        diff,
                        cursor_idx: 0,
                    });
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
            let current = state
                .get_active_panel()
                .filter_mask
                .clone()
                .unwrap_or_default();
            state.active_popup = Some(PopupType::FilePanelFilterPrompt { input: current });
        }
        Action::TaskList => {
            let tasks = get_process_list();
            state.active_popup = Some(PopupType::TaskListDialog {
                tasks,
                cursor_idx: 0,
            });
        }

        Action::SaveSetup => {
            state.active_popup = Some(PopupType::SaveSetupConfirm);
        }
        Action::SystemSettings => {
            state.active_popup = Some(PopupType::ConfigurationDialog {
                active_tab: 0,
                cursor_idx: 0,
                editing_value: false,
                edit_buffer: String::new(),
                settings: context.config.settings.clone(),
            });
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
                state.active_popup = Some(PopupType::Info(format!(
                    "No folder shortcut assigned to Ctrl+Alt+{}",
                    n
                )));
            }
        }

        // ── Stubs ─────────────────────────────────────────────────────────────
        Action::PluginMenu => {
            state.active_popup = Some(PopupType::Info(
                "Plugin system: not yet implemented.".to_string(),
            ));
        }
        Action::ScreensList => {
            state.active_popup = Some(PopupType::Info(
                "Screens list: not yet implemented.".to_string(),
            ));
        }
        Action::VideoMode => {
            state.active_popup = Some(PopupType::Info(
                "Video mode: resize your terminal manually.".to_string(),
            ));
        }
    }
    Ok(())
}

/// Suspends raw mode **in-place**, runs a shell command natively, then re-enables raw mode.
/// Does NOT drop/recreate TerminalBackend to avoid double-restore.
pub fn execute_shell_command(
    command_str: &str,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
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
pub fn execute_external_command(
    _target_path: &Path,
    utility_command: &str,
    terminal_backend: &mut TerminalBackend,
) -> Result<()> {
    // Suspend TUI
    terminal_backend.terminal.flush()?;
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, Show)?;

    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    let flag = if cfg!(target_os = "windows") {
        "/c"
    } else {
        "-c"
    };
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
