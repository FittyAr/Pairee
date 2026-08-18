mod git;
mod help;
mod plugins;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PanelViewMode, PopupType};
use crate::app::sys_helpers::{build_info_panel_lines, get_hotlist_bookmarks, get_process_list};
use crate::config::localization::t;
use crate::keybindings::Action;

/// Handles UI, settings, and other configuration actions. Returns `true` if the action was handled.
pub async fn handle_ui_settings_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    match action {
        Action::About => {
            help::open_about(state);
            true
        }
        Action::Help => {
            help::open_help(state).await;
            true
        }
        Action::UserMenu => {
            state.dialogs.replace(PopupType::UserMenu { cursor_idx: 0 });
            true
        }
        Action::Menu => {
            if let Some(PopupType::Menu { .. }) = state.dialogs.top() {
                state.dialogs.clear();
            } else {
                let active_item_idx = if context.config.settings.auto_drop_menu {
                    Some(0)
                } else {
                    None
                };
                state.dialogs.replace(PopupType::Menu {
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
                    t("ctx_menu_view"),
                    t("ctx_menu_edit"),
                    t("ctx_menu_copy"),
                    t("ctx_menu_move"),
                    t("ctx_menu_delete"),
                    t("ctx_menu_compress"),
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
                    items.push(t("ctx_menu_extract"));
                }
                state.dialogs.replace(PopupType::ContextMenu {
                    items,
                    cursor_idx: 0,
                });
            }
            true
        }
        Action::Quit => {
            if context.config.settings.confirmations.confirm_quit {
                state.dialogs.replace(PopupType::ConfirmQuit);
            } else {
                state.should_quit = true;
            }
            true
        }
        Action::ToggleHidden => {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            context.config.save_logging();
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::FocusCli => {
            state.cli_input.push(' ');
            state.cli_input.clear();
            true
        }
        Action::Unfocus => {
            state.dialogs.clear();
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
            state.panels.left_visible = !state.panels.left_visible;
            true
        }
        Action::TogglePanelRight => {
            state.panels.right_visible = !state.panels.right_visible;
            true
        }
        Action::ToggleBothPanels => {
            state.panels.both_hidden = !state.panels.both_hidden;
            true
        }
        Action::ToggleLongNames => {
            let panel = state.get_active_panel_mut();
            panel.show_long_names = !panel.show_long_names;
            true
        }
        Action::InfoPanel => {
            let lines = build_info_panel_lines(state);
            state.dialogs.replace(PopupType::InfoPanel { lines });
            true
        }
        Action::QuickView => {
            state.panels.quick_view_active = !state.panels.quick_view_active;
            if !state.panels.quick_view_active {
                if let Some(PopupType::QuickViewPanel(_)) = state.dialogs.top() {
                    state.dialogs.clear();
                }
            } else {
                state.update_quick_view();
            }
            true
        }
        Action::SortModes => {
            let current = state.get_active_panel().sort_field;
            let reverse = state.get_active_panel().sort_reverse;
            state.dialogs.replace(PopupType::SortModesDialog {
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
            let left = state.panels.left.current_path.clone();
            let right = state.panels.right.current_path.clone();
            match crate::fs::compare_directories(&left, &right) {
                Ok(diff) => {
                    for entry in &diff {
                        if entry.status != crate::fs::CompareStatus::Equal
                            && let Some(e) = state
                                .panels
                                .left
                                .entries
                                .iter()
                                .find(|e| e.name == entry.name)
                            && state.panels.left.selected_paths.insert(e.path.clone())
                        {
                            state.panels.left.selection_order.push(e.path.clone());
                        }
                    }
                    state.dialogs.replace(PopupType::CompareFoldersResult {
                        diff,
                        cursor_idx: 0,
                    });
                }
                Err(e) => {
                    state.dialogs.replace(PopupType::Error(
                        t("error_compare_failed").replace("{}", &e.to_string()),
                    ));
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
                    state.dialogs.replace(PopupType::Error(
                        t("error_read_usermenu_failed").replace("{}", &e.to_string()),
                    ));
                }
            }
            true
        }
        Action::FileAssociations => {
            let config = crate::config::associations::AssociationsConfig::load();
            state.dialogs.replace(PopupType::FileAssociationsDialog {
                rules: config.rules,
                cursor_idx: 0,
                editing_idx: None,
                editing_field: 0,
                edit_buffer: String::new(),
                original_rule: None,
            });
            true
        }
        Action::FolderShortcutsConfig => {
            let bookmarks = get_hotlist_bookmarks();
            state.dialogs.replace(PopupType::Hotlist {
                bookmarks,
                cursor_idx: 0,
            });
            true
        }
        Action::FilePanelFilter => {
            let active = state.get_active_panel();
            let current = active.filter_mask.clone().unwrap_or_default();
            state
                .dialogs
                .replace(PopupType::FilePanelFilterPrompt { input: current });
            true
        }
        Action::QuickFilter => {
            let active = state.get_active_panel();
            let current = active.quick_filter_mask.clone().unwrap_or_default();
            let original_mask = active.quick_filter_mask.clone();
            let original_cursor = active.cursor_index;
            state.dialogs.replace(PopupType::QuickFilterPrompt {
                input: current,
                original_mask,
                original_cursor,
            });
            true
        }
        Action::TaskList => {
            let tasks = get_process_list();
            state.dialogs.replace(PopupType::TaskListDialog {
                tasks,
                cursor_idx: 0,
                filter_query: String::new(),
                is_filtering: false,
            });
            true
        }
        Action::SaveSetup => {
            state.dialogs.replace(PopupType::SaveSetupConfirm);
            true
        }
        Action::SystemSettings => {
            state.dialogs.replace(PopupType::ConfigurationDialog {
                active_tab: 0,
                cursor_idx: 0,
                editing_value: false,
                edit_buffer: String::new(),
                settings: Box::new(context.config.settings.clone()),
                focus_on_tabs: true,
            });
            true
        }
        Action::FindFile => {
            let root = state.get_active_panel().current_path.clone();
            state.dialogs.replace(PopupType::SearchPrompt {
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
            plugins::open_plugin_menu(state, context);
            true
        }
        Action::ScreensList => {
            let suspended = state.dialogs.take();
            state.dialogs.replace(PopupType::ScreensMenu {
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
            state.dialogs.replace(PopupType::Info(t("video_mode_hint")));
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
        Action::OpenGitPanel => git::open_git_panel(state, context),
        Action::CheckForUpdates => {
            if let Some(info) = state.update.available.clone() {
                // Re-open the popup with existing info
                state
                    .dialogs
                    .replace(crate::app::state::PopupType::UpdateAvailable {
                        info,
                        cursor_idx: 0,
                        install_progress: None,
                        error: None,
                        scroll_y: 0,
                    });
            } else {
                // Force a fresh check (bypass cache by deleting cache file first)
                let cache = crate::config::paths::get_config_dir().join("update_cache.json");
                let _ = std::fs::remove_file(&cache);
                let (tx, rx) = tokio::sync::oneshot::channel();
                crate::update::checker::UpdateChecker::check_in_background(tx);
                state.update.check_rx = Some(rx);
                state.update.status = crate::update::UpdateStatus::Checking;
                state
                    .dialogs
                    .replace(crate::app::state::PopupType::Info(t("update_checking")));
            }
            true
        }
        Action::InstallDevPlugin => plugins::install_dev_plugin(state, context),
        Action::CommandPalette => {
            crate::app::actions::command_palette::open_palette(state);
            true
        }
        Action::ToggleTransferPanel => {
            if let Some(ref mut ts) = state.transfer {
                match ts.view_mode {
                    crate::app::state::TransferViewMode::Hidden
                    | crate::app::state::TransferViewMode::Minimized => {
                        ts.view_mode = crate::app::state::TransferViewMode::Expanded;
                        state.dialogs.replace(PopupType::TransferPanel);
                    }
                    crate::app::state::TransferViewMode::Expanded => {
                        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                        state.dialogs.clear();
                    }
                }
            } else {
                state
                    .dialogs
                    .replace(PopupType::Info(t("transfer_no_active")));
            }
            true
        }
        _ => false,
    }
}
