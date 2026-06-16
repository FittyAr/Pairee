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
                    if let Ok(exe) = std::env::current_exe() {
                        if let Some(parent) = exe.parent() {
                            let alt_path = parent.join(path_str);
                            if alt_path.exists() {
                                path = alt_path;
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
                add_doc("Guía de Arquitectura", "docs/technical/architecture_es.md");
            } else {
                add_doc("Features Reference", "help/features_en.md");
                add_doc("User Guide", "help/user_guide_en.md");
                add_doc(
                    "Technical Architecture",
                    "docs/technical/architecture_en.md",
                );
            }

            state.active_popup = Some(PopupType::Help {
                mode: 0,
                docs,
                cursor_idx: 0,
                scroll_y: 0,
                active_content: None,
            });
            true
        }
        Action::UserMenu => {
            state.active_popup = Some(PopupType::UserMenu);
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
            state.active_popup = Some(PopupType::Info(
                "Edit user menu: open UserMenu config file with default editor.".to_string(),
            ));
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
        _ => false,
    }
}
