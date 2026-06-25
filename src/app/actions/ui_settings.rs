use crate::app::context::AppContext;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::app::sys_helpers::{build_info_panel_lines, get_hotlist_bookmarks, get_process_list};
use crate::keybindings::Action;

/// Handles UI, settings, and other configuration actions. Returns `true` if the action was handled.
pub fn handle_ui_settings_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    match action {
        Action::Help => {
            let mut docs = Vec::new();
            let mut add_doc = |title: &str, path_str: &str| {
                let mut path = std::path::PathBuf::from(path_str);
                if !path.exists() {
                    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
                        let manifest_path = std::path::PathBuf::from(manifest_dir).join(path_str);
                        if manifest_path.exists() {
                            path = manifest_path;
                        }
                    }
                }
                if !path.exists() {
                    if let Ok(exe) = std::env::current_exe() {
                        let mut current = exe.parent();
                        while let Some(dir) = current {
                            let candidate = dir.join(path_str);
                            if candidate.exists() {
                                path = candidate;
                                break;
                            }
                            current = dir.parent();
                        }
                    }
                }
                if !path.exists() {
                    let config_path = crate::config::paths::get_config_dir().join(path_str);
                    if config_path.exists() {
                        path = config_path;
                    }
                }
                if !path.exists() {
                    if let Some(share_dir) = crate::config::paths::get_system_share_dir() {
                        let share_path = share_dir.join(path_str);
                        if share_path.exists() {
                            path = share_path;
                        }
                    }
                }
                docs.push((title.to_string(), path));
            };

            let is_spanish = context
                .config
                .settings
                .language
                .to_lowercase()
                .contains("es")
                || context
                    .config
                    .settings
                    .language
                    .to_lowercase()
                    .contains("spanish");

            if is_spanish {
                add_doc("Manual de Funciones", "help/features_es.md");
                add_doc("Guía de Usuario", "help/user_guide_es.md");
                add_doc("Conexión SSH y SFTP", "help/ssh_sftp_es.md");
                add_doc("Integración con Git", "help/git_integration_es.md");
                add_doc("Configuración de Ajustes", "help/configuration_details_es.md");
                add_doc("Atajos de Teclado", "help/keyboard_shortcuts_es.md");
                add_doc("Guía de Arquitectura", "docs/technical/architecture_es.md");
            } else {
                add_doc("Features Reference", "help/features_en.md");
                add_doc("User Guide", "help/user_guide_en.md");
                add_doc("SSH & SFTP Remote Connections", "help/ssh_sftp_en.md");
                add_doc("Git Integration", "help/git_integration_en.md");
                add_doc("Configuration Settings", "help/configuration_details_en.md");
                add_doc("Keyboard Shortcuts", "help/keyboard_shortcuts_en.md");
                add_doc(
                    "Technical Architecture",
                    "docs/technical/architecture_en.md",
                );
            }

            let first_content = if !docs.is_empty() {
                std::fs::read_to_string(&docs[0].1).ok()
            } else {
                None
            };

            state.active_popup = Some(PopupType::Help {
                mode: 0,
                docs,
                cursor_idx: 0,
                scroll_y: 0,
                active_content: first_content,
            });
            true
        }
        Action::UserMenu => {
            state.active_popup = Some(PopupType::UserMenu { cursor_idx: 0 });
            true
        }
        Action::Menu => {
            if let Some(PopupType::Menu { .. }) = state.active_popup {
                state.active_popup = None;
            } else {
                let active_item_idx = if context.config.settings.auto_drop_menu {
                    Some(0)
                } else {
                    None
                };
                state.active_popup = Some(PopupType::Menu {
                    active_menu_idx: 0,
                    active_item_idx,
                    active_submenu_idx: None,
                    active_submenu_item_idx: None,
                });
            }
            true
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
            true
        }
        Action::Quit => {
            if context.config.settings.confirmations.confirm_quit {
                state.active_popup = Some(PopupType::ConfirmQuit);
            } else {
                state.should_quit = true;
            }
            true
        }
        Action::ToggleHidden => {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            let _ = context.config.save();
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::FocusCli => {
            state.cli_input.push(' ');
            state.cli_input.clear();
            true
        }
        Action::Unfocus => {
            state.active_popup = None;
            state.cli_input.clear();
            state.fkeys_modifier_override = None;
            true
        }
        Action::Refresh | Action::RereadPanel => {
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::PanelViewBrief => {
            state.get_active_panel_mut().view_mode = PanelViewMode::Brief;
            true
        }
        Action::PanelViewMedium => {
            state.get_active_panel_mut().view_mode = PanelViewMode::Medium;
            true
        }
        Action::PanelViewFull => {
            state.get_active_panel_mut().view_mode = PanelViewMode::Full;
            true
        }
        Action::PanelViewWide => {
            state.get_active_panel_mut().view_mode = PanelViewMode::Wide;
            true
        }
        Action::PanelViewDetailed => {
            state.get_active_panel_mut().view_mode = PanelViewMode::Detailed;
            true
        }
        Action::PanelViewDescriptions => {
            state.get_active_panel_mut().view_mode = PanelViewMode::Descriptions;
            true
        }
        Action::PanelViewFileOwners => {
            state.get_active_panel_mut().view_mode = PanelViewMode::FileOwners;
            true
        }
        Action::PanelViewFileLinks => {
            state.get_active_panel_mut().view_mode = PanelViewMode::FileLinks;
            true
        }
        Action::PanelViewAltFull => {
            state.get_active_panel_mut().view_mode = PanelViewMode::AltFull;
            true
        }
        Action::TogglePanelLeft => {
            state.left_panel_visible = !state.left_panel_visible;
            true
        }
        Action::TogglePanelRight => {
            state.right_panel_visible = !state.right_panel_visible;
            true
        }
        Action::ToggleBothPanels => {
            state.both_panels_hidden = !state.both_panels_hidden;
            true
        }
        Action::ToggleLongNames => {
            let panel = state.get_active_panel_mut();
            panel.show_long_names = !panel.show_long_names;
            true
        }
        Action::InfoPanel => {
            let lines = build_info_panel_lines(state);
            state.active_popup = Some(PopupType::InfoPanel { lines });
            true
        }
        Action::QuickView => {
            state.quick_view_active = !state.quick_view_active;
            if !state.quick_view_active {
                if let Some(PopupType::QuickViewPanel { .. }) = state.active_popup {
                    state.active_popup = None;
                }
            } else {
                state.update_quick_view();
            }
            true
        }
        Action::SortModes => {
            let current = state.get_active_panel().sort_field;
            let reverse = state.get_active_panel().sort_reverse;
            state.active_popup = Some(PopupType::SortModesDialog {
                current,
                reverse,
                cursor_idx: 0,
            });
            true
        }
        Action::ToggleSortReverse => {
            let current = state.get_active_panel().sort_reverse;
            state.get_active_panel_mut().sort_reverse = !current;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByName => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Name;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByExtension => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Extension;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByWriteTime | Action::SortByCreationTime | Action::SortByAccessTime => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Date;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortBySize => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Size;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortUnsorted => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Unsorted;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::SortByDescription | Action::SortByOwner => {
            state.get_active_panel_mut().sort_field = crate::app::state::SortField::Name;
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
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
                                if state.left_panel.selected_paths.insert(e.path.clone()) {
                                    state.left_panel.selection_order.push(e.path.clone());
                                }
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
            true
        }
        Action::EditUserMenu => {
            let path = crate::config::paths::get_config_dir().join("usermenu.toml");
            if !path.exists() {
                let default_template = r#"# Pairee User Custom Commands Menu
#
# Define your own custom commands here.
# Format:
# [commands]
# "Key" = "Command"
#
# Examples:
# "1" = "cargo build"
# "2" = "git status"
# "3" = "echo 'Hello World!'"
# "4" = "systemctl status docker"
"#;
                let _ = std::fs::write(&path, default_template);
            }
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                    state.push_screen(crate::app::state::Screen::Editor(
                        crate::app::state::types::EditorState {
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
                            last_case_sensitive: false,
                        },
                    ));
                }
                Err(e) => {
                    state.active_popup = Some(PopupType::Error(format!(
                        "Failed to read user menu config: {}",
                        e
                    )));
                }
            }
            true
        }
        Action::FileAssociations => {
            let config = crate::config::associations::AssociationsConfig::load();
            state.active_popup = Some(PopupType::FileAssociationsDialog {
                rules: config.rules,
                cursor_idx: 0,
            });
            true
        }
        Action::FolderShortcutsConfig => {
            let bookmarks = get_hotlist_bookmarks();
            state.active_popup = Some(PopupType::Hotlist {
                bookmarks,
                cursor_idx: 0,
            });
            true
        }
        Action::FilePanelFilter => {
            let current = state
                .get_active_panel()
                .filter_mask
                .clone()
                .unwrap_or_default();
            state.active_popup = Some(PopupType::FilePanelFilterPrompt { input: current });
            true
        }
        Action::TaskList => {
            let tasks = get_process_list();
            state.active_popup = Some(PopupType::TaskListDialog {
                tasks,
                cursor_idx: 0,
            });
            true
        }
        Action::SaveSetup => {
            state.active_popup = Some(PopupType::SaveSetupConfirm);
            true
        }
        Action::SystemSettings => {
            state.active_popup = Some(PopupType::ConfigurationDialog {
                active_tab: 0,
                cursor_idx: 0,
                editing_value: false,
                edit_buffer: String::new(),
                settings: context.config.settings.clone(),
                focus_on_tabs: true,
            });
            true
        }
        Action::FindFile => {
            let root = state.get_active_panel().current_path.clone();
            state.active_popup = Some(PopupType::SearchPrompt {
                query: String::new(),
                content_query: String::new(),
                search_root: root,
                case_sensitive: false,
                search_target: crate::fs::search::SearchTarget::Any,
                cursor_idx: 0,
            });
            true
        }
        Action::PluginMenu => {
            state.active_popup = Some(PopupType::Info(
                "Plugin system: not yet implemented.".to_string(),
            ));
            true
        }
        Action::ScreensList => {
            let suspended = state.active_popup.take();
            state.active_popup = Some(PopupType::ScreensMenu {
                cursor_idx: state.active_screen_idx,
                suspended_popup: suspended.map(Box::new),
            });
            true
        }
        Action::NextScreen => {
            state.next_screen();
            true
        }
        Action::PrevScreen => {
            state.prev_screen();
            true
        }
        Action::VideoMode => {
            state.active_popup = Some(PopupType::Info(
                "Video mode: resize your terminal manually.".to_string(),
            ));
            true
        }
        Action::CycleFKeysModifiers => {
            use crossterm::event::KeyModifiers;
            state.fkeys_modifier_override = match state.fkeys_modifier_override {
                None => Some(KeyModifiers::CONTROL),
                Some(KeyModifiers::CONTROL) => Some(KeyModifiers::ALT),
                Some(KeyModifiers::ALT) => None,
                _ => None,
            };
            true
        }
        Action::OpenGitPanel => {
            if !context.config.settings.git_enabled {
                return false;
            }
            let panel_path = state.get_active_panel().current_path.clone();
            match crate::git::repo::find_repo(&panel_path) {
                Some(repo) => {
                    let repo_path =
                        crate::git::repo::get_workdir(&repo).unwrap_or_else(|| panel_path.clone());
                    let current_branch = repo
                        .head()
                        .ok()
                        .and_then(|h| h.shorthand().ok().map(|s| s.to_string()))
                        .unwrap_or_else(|| "(detached HEAD)".to_string());
                    let limit = context.config.settings.git_log_limit as usize;
                    let status_entries = crate::git::status::get_status(&repo);
                    let log_entries = crate::git::log::get_log(&repo, limit);
                    let branch_entries = crate::git::branches::get_branches(&repo);
                    state.active_popup = Some(crate::app::state::PopupType::GitPanel {
                        repo_path,
                        active_tab: 0,
                        cursor_idx: 0,
                        scroll: 0,
                        status_entries,
                        log_entries,
                        branch_entries,
                        current_branch,
                        pending_action: None,
                    });
                }
                None => {
                    state.active_popup = Some(crate::app::state::PopupType::Error(
                        crate::config::localization::t("git_not_a_repo"),
                    ));
                }
            }
            true
        }
        _ => false,
    }
}
