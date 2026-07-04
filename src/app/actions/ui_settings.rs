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
            state.active_popup = Some(PopupType::About { scroll_y: 0 });
            true
        }
        Action::Help => {
            let mut docs = Vec::new();

            let resolve_help_dir = || -> Option<std::path::PathBuf> {
                // Try CARGO_MANIFEST_DIR first
                if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
                    let manifest_path = std::path::PathBuf::from(manifest_dir).join("help");
                    if manifest_path.exists() && manifest_path.is_dir() {
                        return Some(manifest_path);
                    }
                }
                // Try current executable parents
                if let Ok(exe) = std::env::current_exe() {
                    let mut current = exe.parent();
                    while let Some(dir) = current {
                        let candidate = dir.join("help");
                        if candidate.exists() && candidate.is_dir() {
                            return Some(candidate);
                        }
                        current = dir.parent();
                    }
                }
                // Try config dir
                let config_path = crate::config::paths::get_config_dir().join("help");
                if config_path.exists() && config_path.is_dir() {
                    return Some(config_path);
                }
                // Try system share dir
                if let Some(share_dir) = crate::config::paths::get_system_share_dir() {
                    let share_path = share_dir.join("help");
                    if share_path.exists() && share_path.is_dir() {
                        return Some(share_path);
                    }
                }
                None
            };

            let lang_code = crate::config::localization::get_active_language_code();
            let mut help_dir = resolve_help_dir().map(|r| r.join(&lang_code));
            if help_dir.is_none() || !help_dir.as_ref().unwrap().exists() {
                help_dir = resolve_help_dir().map(|r| r.join("en"));
            }

            if let Some(ref dir_path) = help_dir {
                if let Ok(entries) = std::fs::read_dir(dir_path) {
                    let mut files = Vec::new();
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                if ext.to_lowercase() == "md" {
                                    files.push(path);
                                }
                            }
                        }
                    }
                    // Sort files alphabetically by filename
                    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

                    for path in files {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let translation_key = format!("help_title_{}", stem);
                            let title = crate::config::localization::t(&translation_key);
                            let display_title = if title == translation_key {
                                stem.split('_')
                                    .map(|word| {
                                        let mut chars = word.chars();
                                        match chars.next() {
                                            None => String::new(),
                                            Some(first) => {
                                                first.to_uppercase().collect::<String>()
                                                    + chars.as_str()
                                            }
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join(" ")
                            } else {
                                title
                            };
                            docs.push((display_title, path));
                        }
                    }
                }
            }

            let mut plugin_docs = Vec::new();
            let loaded_plugins = crate::plugin::registry::get_loaded_plugins().await;
            for p in loaded_plugins {
                let help_dir = p.path.join("help");
                if help_dir.exists() && help_dir.is_dir() {
                    let lang_code = crate::config::localization::get_active_language_code();
                    let mut help_path = help_dir.join(format!("{}.md", &lang_code));
                    if !help_path.exists() {
                        let default_lang = p.manifest.default_language.as_deref().unwrap_or("en");
                        help_path = help_dir.join(format!("{}.md", default_lang));
                    }
                    if help_path.exists() && help_path.is_file() {
                        plugin_docs.push((p.manifest.name.clone(), help_path));
                    }
                }
            }

            let first_content = if !docs.is_empty() {
                std::fs::read_to_string(&docs[0].1).ok()
            } else {
                None
            };

            state.active_popup = Some(PopupType::Help {
                mode: 0,
                docs,
                plugin_docs,
                active_tab: 0,
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
            let active = state.get_active_panel();
            let current = active.filter_mask.clone().unwrap_or_default();
            state.active_popup = Some(PopupType::FilePanelFilterPrompt { input: current });
            true
        }
        Action::QuickFilter => {
            let active = state.get_active_panel();
            let current = active.quick_filter_mask.clone().unwrap_or_default();
            let original_mask = active.quick_filter_mask.clone();
            let original_cursor = active.cursor_index;
            state.active_popup = Some(PopupType::QuickFilterPrompt {
                input: current,
                original_mask,
                original_cursor,
            });
            true
        }
        Action::TaskList => {
            let tasks = get_process_list();
            state.active_popup = Some(PopupType::TaskListDialog {
                tasks,
                cursor_idx: 0,
                filter_query: String::new(),
                is_filtering: false,
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
            // Open the popup immediately so the UI stays responsive while we
            // fetch the registry index and assemble the installed list in the
            // background. The status line + spinner shows progress to the user.
            state.active_popup = Some(PopupType::PluginMenu {
                active_tab: 0,
                cursor_idx: 0,
                installed: Vec::new(),
                all_registry: Vec::new(),
                registry: Vec::new(),
                search_query: String::new(),
                is_searching: false,
                editing_query: false,
                dev_results: String::new(),
                dev_wizard_step: 0,
                dev_wizard_data: Vec::new(),
                installed_loading: true,
                installed_loading_status: t("plugin_dev_progress_loading_index"),
                dev_loading: false,
                dev_loading_status: String::new(),
                dev_loading_progress: None,
            });

            // Snapshot the data we need from `context` (which is borrowed
            // mutably) so the background task does not capture a reference
            // to it.
            let plugins_settings = context.config.settings.plugins.clone();
            let tx = crate::plugin::PluginManager::get_sender();
            tokio::spawn(async move {
                let lock = crate::plugin::updater::read_lockfile();
                let index = crate::plugin::updater::fetch_index().await.ok();
                let mut installed = Vec::new();
                for (name, info) in &lock.plugins {
                    let trusted = plugins_settings
                        .get(name)
                        .map(|p| p.trusted)
                        .unwrap_or(false);

                    let update_available = if let Some(ref idx) = index {
                        if let Some(reg_plugin) = idx.plugins.get(name) {
                            if reg_plugin.version != info.version {
                                Some(reg_plugin.version.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    installed.push((
                        name.clone(),
                        info.version.clone(),
                        info.pinned,
                        trusted,
                        update_available,
                    ));
                }
                // Build the full registry list (all available plugins) so the
                // Search tab shows results immediately without requiring a query.
                let registry: Vec<(String, String, String, String)> = index
                    .as_ref()
                    .map(|idx| {
                        let mut list: Vec<_> = idx
                            .plugins
                            .iter()
                            .map(|(name, p)| {
                                (
                                    name.clone(),
                                    p.version.clone(),
                                    p.description.clone().unwrap_or_default(),
                                    p.author.clone().unwrap_or_default(),
                                )
                            })
                            .collect();
                        list.sort_by(|a, b| a.0.cmp(&b.0));
                        list
                    })
                    .unwrap_or_default();
                let _ = tx
                    .send(crate::plugin::manager::PluginRequest::PluginMenuLoaded {
                        installed,
                        registry,
                    })
                    .await;
            });

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
        Action::CheckForUpdates => {
            if let Some(info) = state.update_available.clone() {
                // Re-open the popup with existing info
                state.active_popup = Some(crate::app::state::PopupType::UpdateAvailable {
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
                state.update_check_rx = Some(rx);
                state.update_status = crate::update::UpdateStatus::Checking;
                state.active_popup = Some(crate::app::state::PopupType::Info(
                    "Checking for updates...".to_string(),
                ));
            }
            true
        }
        Action::InstallDevPlugin => {
            if !context.config.settings.plugins_developer_mode {
                return false;
            }
            let active_panel = state.get_active_panel();
            let current_dir = &active_panel.current_path;

            let mut target_dir = current_dir.clone();
            if let Some(entry) = active_panel.entries.get(active_panel.cursor_index) {
                if entry.path.is_dir() && entry.path.join("manifest.toml").exists() {
                    target_dir = entry.path.clone();
                }
            }

            let manifest_path = target_dir.join("manifest.toml");
            if manifest_path.exists() {
                if let Ok(manifest_content) = std::fs::read_to_string(&manifest_path) {
                    if let Ok(manifest) =
                        crate::plugin::loader::PluginManifest::parse(&manifest_content)
                    {
                        let name = manifest.name.clone();
                        let version = manifest.version.clone();
                        let dest_dir = crate::config::paths::get_config_dir()
                            .join("plugins")
                            .join(format!("{}.pairee", name));

                        let _ = std::fs::create_dir_all(&dest_dir);
                        let mut success = true;
                        if let Ok(entries) = std::fs::read_dir(&target_dir) {
                            for entry in entries.filter_map(Result::ok) {
                                let path = entry.path();
                                if path.is_file() {
                                    if let Some(filename) = path.file_name() {
                                        let _ = std::fs::copy(&path, dest_dir.join(filename));
                                    }
                                } else if path.is_dir()
                                    && path.file_name().map(|n| n == "lang").unwrap_or(false)
                                {
                                    let lang_dest = dest_dir.join("lang");
                                    let _ = std::fs::create_dir_all(&lang_dest);
                                    if let Ok(lang_entries) = std::fs::read_dir(&path) {
                                        for le in lang_entries.filter_map(Result::ok) {
                                            if le.path().is_file() {
                                                if let Some(fn_lang) = le.path().file_name() {
                                                    let _ = std::fs::copy(
                                                        le.path(),
                                                        lang_dest.join(fn_lang),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            success = false;
                        }

                        if success {
                            let mut lock = crate::plugin::updater::read_lockfile();
                            let mut files_hash = std::collections::HashMap::new();
                            for (rel, p) in crate::plugin::loader::get_plugin_files(&dest_dir) {
                                if let Ok(h) = crate::update::downloader::compute_sha256(&p) {
                                    files_hash.insert(rel, h);
                                }
                            }
                            lock.plugins.insert(
                                name.clone(),
                                crate::plugin::updater::PinnedPlugin {
                                    version,
                                    pinned: false,
                                    files: files_hash,
                                },
                            );
                            let _ = crate::plugin::updater::write_lockfile(&lock);

                            state.active_popup = Some(crate::app::state::PopupType::Info(format!(
                                "✓ Local plugin '{}' installed successfully to:\n{:?}",
                                name, dest_dir
                            )));
                        } else {
                            state.active_popup = Some(crate::app::state::PopupType::Error(
                                format!("Failed to copy plugin files for '{}'.", name),
                            ));
                        }
                    }
                }
            }
            true
        }
        _ => false,
    }
}
