use super::actions::Action;
use crate::config::paths;
use std::collections::HashMap;

// ── TOML deserialization helper ────────────────────────────────────────────────

/// Intermediate struct used to deserialize a preset TOML file.
/// The file must have a `[bindings]` table mapping action names to key strings.
#[derive(serde::Deserialize)]
struct PresetFile {
    bindings: HashMap<String, String>,
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Returns the key-to-action bindings for the given preset name.
///
/// Load order:
/// 1. Try to read `<keymaps_dir>/<preset>.toml` from the user config folder.
/// 2. Fall back to the compiled-in defaults if the file is missing or invalid,
///    logging a warning in either failure case.
pub fn get_preset_bindings(preset: &str) -> HashMap<String, Action> {
    if let Some(file_bindings) = load_preset_from_file(preset) {
        return file_bindings;
    }
    // Compiled-in fallback so the app is never broken by a missing/corrupt file.
    log::warn!(
        "Keybinding preset file '{}' not found or invalid — using built-in defaults.",
        preset
    );
    get_builtin_preset_bindings(preset)
}

/// Attempts to load a preset from `<keymaps_dir>/<preset>.toml`.
/// Returns `None` if the file does not exist or cannot be parsed.
fn load_preset_from_file(preset: &str) -> Option<HashMap<String, Action>> {
    let path = paths::get_keymaps_dir().join(format!("{}.toml", preset));
    if !path.exists() {
        return None;
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to read keybinding preset '{}': {}", preset, e);
            return None;
        }
    };
    let preset_file: PresetFile = match toml::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to parse keybinding preset '{}': {}", preset, e);
            return None;
        }
    };

    // Start with default built-in bindings for this preset
    let mut map = get_builtin_preset_bindings(preset);

    let mut cleared_actions = std::collections::HashSet::new();

    for (action_name, key_str) in preset_file.bindings {
        if let Some(action) = parse_action_name(&action_name) {
            // Remove any existing default bindings for this action only once
            if cleared_actions.insert(action) {
                map.retain(|_, v| *v != action);
            }
            for key in key_str.split(',') {
                let trimmed = key.trim();
                if !trimmed.is_empty() {
                    map.insert(normalize_key_string(trimmed), action);
                }
            }
        } else {
            log::warn!(
                "Preset '{}': unknown action '{}' — skipped.",
                preset,
                action_name
            );
        }
    }
    Some(map)
}

/// Returns the content of a preset TOML file as a string, generated from the
/// built-in defaults. Used by `AppConfig::load_or_create` to seed the keymaps
/// directory on first run if the file does not yet exist.
pub fn get_builtin_preset_toml(preset: &str) -> String {
    let bindings = get_builtin_preset_bindings(preset);

    // Group keys by action to preserve all default keybindings deterministically
    let mut action_to_keys: HashMap<String, Vec<String>> = HashMap::new();
    for (key_str, action) in &bindings {
        let action_name = action_to_name(*action);
        action_to_keys
            .entry(action_name)
            .or_default()
            .push(key_str.clone());
    }

    let mut lines: Vec<String> = action_to_keys
        .into_iter()
        .map(|(action_name, mut keys)| {
            keys.sort();
            let keys_str = keys.join(", ");
            format!("{:<30} = {:?}", action_name, keys_str)
        })
        .collect();
    lines.sort();

    format!(
        "# Pairee keybinding preset: {}\n# Edit this file to customise your shortcuts.\n# Available actions: see the Pairee documentation.\n\n[bindings]\n{}\n",
        preset,
        lines.join("\n")
    )
}

// ── Built-in fallback presets ──────────────────────────────────────────────────

/// Returns hardcoded default bindings for a preset.
/// These are the fallback used when a preset file is missing.
pub fn get_builtin_preset_bindings(preset: &str) -> HashMap<String, Action> {
    let mut map = HashMap::new();
    match preset.to_lowercase().as_str() {
        "neovim" | "vim" => {
            insert_common_norton_bindings(&mut map);
            // Neovim-style navigation overrides
            map.insert("k".to_string(), Action::MoveUp);
            map.insert("j".to_string(), Action::MoveDown);
            map.insert("Ctrl+u".to_string(), Action::PageUp);
            map.insert("Ctrl+d".to_string(), Action::PageDown);
            map.insert("g".to_string(), Action::GoToTop);
            map.insert("G".to_string(), Action::GoToBottom);
            map.insert("l".to_string(), Action::Execute);
            map.insert("h".to_string(), Action::GoParent);
            map.insert("v".to_string(), Action::SelectItem);
            map.insert("y".to_string(), Action::Copy);
            map.insert("m".to_string(), Action::Move);
            map.insert("d".to_string(), Action::Delete);
            map.insert(":".to_string(), Action::FocusCli);
            map.insert("/".to_string(), Action::FindFile);
            map.insert("Ctrl+c".to_string(), Action::Quit);
        }
        "vscode" | "modern" => {
            insert_common_norton_bindings(&mut map);
            // VS Code / modern overrides
            map.insert("Ctrl+c".to_string(), Action::Copy);
            map.insert("Ctrl+x".to_string(), Action::Move);
            map.insert("Delete".to_string(), Action::Delete);
            map.insert("Ctrl+Shift+n".to_string(), Action::MkDir);
            map.insert("Ctrl+f".to_string(), Action::FindFile);
            map.insert("Ctrl+q".to_string(), Action::Quit);
            map.insert("Ctrl+,".to_string(), Action::SystemSettings);
            map.insert("Ctrl+b".to_string(), Action::ToggleBothPanels);
            map.insert("Shift+F10".to_string(), Action::ContextMenu);
            map.insert("Ctrl+Shift+.".to_string(), Action::ToggleHidden);
            map.insert("Ctrl+Shift+t".to_string(), Action::TreeView);
        }
        _ => {
            // Default "norton" preset — classic NC/Far Manager key layout
            insert_common_norton_bindings(&mut map);
            map.insert("F5".to_string(), Action::Copy);
            map.insert("F6".to_string(), Action::Move);
        }
    }
    map
}

/// Inserts bindings that are identical across all built-in presets.
fn insert_common_norton_bindings(map: &mut HashMap<String, Action>) {
    // ── Navigation ────────────────────────────────────────────────────────────
    map.insert("Up".to_string(), Action::MoveUp);
    map.insert("Down".to_string(), Action::MoveDown);
    map.insert("PageUp".to_string(), Action::PageUp);
    map.insert("PageDown".to_string(), Action::PageDown);
    map.insert("Home".to_string(), Action::GoToTop);
    map.insert("End".to_string(), Action::GoToBottom);
    map.insert("Tab".to_string(), Action::ChangePanel);
    map.insert("Insert".to_string(), Action::SelectItem);
    map.insert("Space".to_string(), Action::SelectItem);
    map.insert("Enter".to_string(), Action::Execute);
    map.insert("Backspace".to_string(), Action::GoParent);

    // ── Panel view modes (Ctrl+1 … Ctrl+9) ───────────────────────────────────
    map.insert("Ctrl+1".to_string(), Action::PanelViewBrief);
    map.insert("Ctrl+2".to_string(), Action::PanelViewMedium);
    map.insert("Ctrl+3".to_string(), Action::PanelViewFull);
    map.insert("Ctrl+4".to_string(), Action::PanelViewWide);
    map.insert("Ctrl+5".to_string(), Action::PanelViewDetailed);
    map.insert("Ctrl+6".to_string(), Action::PanelViewDescriptions);
    map.insert("Ctrl+7".to_string(), Action::PanelViewFileOwners);
    map.insert("Ctrl+8".to_string(), Action::PanelViewFileLinks);
    map.insert("Ctrl+9".to_string(), Action::PanelViewAltFull);

    // ── Panel toggles ─────────────────────────────────────────────────────────
    map.insert("Ctrl+F1".to_string(), Action::TogglePanelLeft);
    map.insert("Ctrl+F2".to_string(), Action::TogglePanelRight);
    map.insert("Ctrl+o".to_string(), Action::ToggleBothPanels);
    map.insert("Ctrl+O".to_string(), Action::ToggleBothPanels);
    map.insert("Ctrl+l".to_string(), Action::InfoPanel);
    map.insert("Ctrl+L".to_string(), Action::InfoPanel);
    map.insert("Ctrl+q".to_string(), Action::QuickView);
    map.insert("Ctrl+Q".to_string(), Action::QuickView);
    map.insert("Ctrl+F3".to_string(), Action::SortByName);
    map.insert("Ctrl+F4".to_string(), Action::SortByExtension);
    map.insert("Ctrl+F5".to_string(), Action::SortByWriteTime);
    map.insert("Ctrl+F6".to_string(), Action::SortBySize);
    map.insert("Ctrl+F7".to_string(), Action::SortUnsorted);
    map.insert("Ctrl+F8".to_string(), Action::SortByCreationTime);
    map.insert("Ctrl+F9".to_string(), Action::SortByAccessTime);
    map.insert("Ctrl+F10".to_string(), Action::SortByDescription);
    map.insert("Ctrl+F11".to_string(), Action::SortByOwner);
    map.insert("Ctrl+F12".to_string(), Action::SortModes);
    map.insert("Ctrl+n".to_string(), Action::ToggleLongNames);
    map.insert("Ctrl+N".to_string(), Action::ToggleLongNames);

    // ── F-key standard actions ────────────────────────────────────────────────
    map.insert("F1".to_string(), Action::Help);
    map.insert("F2".to_string(), Action::UserMenu);
    map.insert("F3".to_string(), Action::View);
    map.insert("F4".to_string(), Action::Edit);
    map.insert("F7".to_string(), Action::MkDir);
    map.insert("F8".to_string(), Action::Delete);
    map.insert("F9".to_string(), Action::Menu);
    map.insert("F10".to_string(), Action::Quit);
    map.insert("F11".to_string(), Action::PluginMenu);
    map.insert("F12".to_string(), Action::ScreensList);
    map.insert("Ctrl+Tab".to_string(), Action::NextScreen);
    map.insert("Ctrl+Shift+Tab".to_string(), Action::PrevScreen);

    // ── Shift+F actions ───────────────────────────────────────────────────────
    map.insert("Shift+F1".to_string(), Action::CompressFiles);
    map.insert("Shift+F2".to_string(), Action::ExtractArchive);
    map.insert("Shift+F3".to_string(), Action::ArchiveCommands);
    map.insert("Shift+F9".to_string(), Action::SaveSetup);
    map.insert("Shift+F11".to_string(), Action::InstallDevPlugin);

    // ── Alt+F actions ─────────────────────────────────────────────────────────
    map.insert("Alt+F1".to_string(), Action::DriveSelectLeft);
    map.insert("Alt+F2".to_string(), Action::DriveSelectRight);
    map.insert("Alt+F3".to_string(), Action::ViewAlt);
    map.insert("Alt+F4".to_string(), Action::Edit);
    map.insert("Alt+F5".to_string(), Action::PrintFile);
    map.insert("Alt+F6".to_string(), Action::CreateLink);
    map.insert("Alt+F7".to_string(), Action::FindFile);
    map.insert("Alt+F8".to_string(), Action::CommandHistory);
    map.insert("Alt+F9".to_string(), Action::VideoMode);
    map.insert("Alt+F10".to_string(), Action::TreeView);
    map.insert("Alt+F11".to_string(), Action::FileViewHistory);
    map.insert("Alt+F12".to_string(), Action::FoldersHistory);

    // ── File operations ───────────────────────────────────────────────────────
    map.insert("Alt+Delete".to_string(), Action::WipeFile);
    map.insert("Ctrl+a".to_string(), Action::FileAttributes);
    map.insert("Ctrl+A".to_string(), Action::FileAttributes);
    map.insert("Ctrl+g".to_string(), Action::ApplyCommand);
    map.insert("Ctrl+G".to_string(), Action::ApplyCommand);
    map.insert("Ctrl+z".to_string(), Action::DescribeFile);
    map.insert("Ctrl+Z".to_string(), Action::DescribeFile);

    // ── Bulk selection ────────────────────────────────────────────────────────
    map.insert("Gray+".to_string(), Action::SelectGroup);
    map.insert("Gray-".to_string(), Action::UnselectGroup);
    map.insert("Gray*".to_string(), Action::InvertSelection);
    map.insert("Ctrl+m".to_string(), Action::RestoreSelection);
    map.insert("Ctrl+M".to_string(), Action::RestoreSelection);

    // ── Commands ──────────────────────────────────────────────────────────────
    map.insert("Ctrl+u".to_string(), Action::SwapPanels);
    map.insert("Ctrl+U".to_string(), Action::SwapPanels);
    map.insert("Ctrl+i".to_string(), Action::FilePanelFilter);
    map.insert("Ctrl+I".to_string(), Action::FilePanelFilter);
    map.insert("Ctrl+f".to_string(), Action::QuickFilter);
    map.insert("Ctrl+F".to_string(), Action::QuickFilter);
    map.insert("f".to_string(), Action::QuickFilter);
    map.insert("F".to_string(), Action::QuickFilter);
    map.insert("Ctrl+w".to_string(), Action::TaskList);
    map.insert("Ctrl+W".to_string(), Action::TaskList);

    // ── General ───────────────────────────────────────────────────────────────
    map.insert("Ctrl+h".to_string(), Action::ToggleHidden);
    map.insert("Ctrl+H".to_string(), Action::ToggleHidden);
    map.insert("Ctrl+r".to_string(), Action::Refresh);
    map.insert("Ctrl+R".to_string(), Action::Refresh);
    map.insert("Ctrl+p".to_string(), Action::CycleFKeysModifiers);
    map.insert("Ctrl+P".to_string(), Action::CycleFKeysModifiers);
    map.insert("Ctrl+Shift+S".to_string(), Action::SshConnect);
    map.insert("Esc".to_string(), Action::Unfocus);
    map.insert("Alt+c".to_string(), Action::CompressFiles);
    map.insert("Alt+e".to_string(), Action::ExtractArchive);
    map.insert("Menu".to_string(), Action::ContextMenu);
    map.insert("Alt+m".to_string(), Action::ContextMenu);
    map.insert("Alt+g".to_string(), Action::OpenGitPanel);
    map.insert("Alt+G".to_string(), Action::OpenGitPanel);

    // ── Folder shortcuts 1–9 (Ctrl+Alt+n) ────────────────────────────────────
    for n in 1u8..=9 {
        map.insert(format!("Ctrl+Alt+{}", n), Action::GoFolderShortcut(n));
    }
}

// ── Action ↔ name bidirectional mapping ────────────────────────────────────────

/// Converts an `Action` variant to its canonical snake_case name.
/// Used when generating a preset TOML from built-in defaults.
fn action_to_name(action: Action) -> String {
    match action {
        Action::MoveUp => "move_up",
        Action::MoveDown => "move_down",
        Action::PageUp => "page_up",
        Action::PageDown => "page_down",
        Action::GoToTop => "go_to_top",
        Action::GoToBottom => "go_to_bottom",
        Action::ChangePanel => "change_panel",
        Action::SelectItem => "select_item",
        Action::Execute => "execute",
        Action::GoParent => "go_parent",
        Action::PanelViewBrief => "panel_view_brief",
        Action::PanelViewMedium => "panel_view_medium",
        Action::PanelViewFull => "panel_view_full",
        Action::PanelViewWide => "panel_view_wide",
        Action::PanelViewDetailed => "panel_view_detailed",
        Action::PanelViewDescriptions => "panel_view_descriptions",
        Action::PanelViewFileOwners => "panel_view_file_owners",
        Action::PanelViewFileLinks => "panel_view_file_links",
        Action::PanelViewAltFull => "panel_view_alt_full",
        Action::TogglePanelLeft => "toggle_panel_left",
        Action::TogglePanelRight => "toggle_panel_right",
        Action::ToggleBothPanels => "toggle_both_panels",
        Action::InfoPanel => "info_panel",
        Action::QuickView => "quick_view",
        Action::SortModes => "sort_modes",
        Action::SortByName => "sort_by_name",
        Action::SortByExtension => "sort_by_extension",
        Action::SortByWriteTime => "sort_by_write_time",
        Action::SortBySize => "sort_by_size",
        Action::SortUnsorted => "sort_unsorted",
        Action::SortByCreationTime => "sort_by_creation_time",
        Action::SortByAccessTime => "sort_by_access_time",
        Action::SortByDescription => "sort_by_description",
        Action::SortByOwner => "sort_by_owner",
        Action::Help => "help",
        Action::About => "about",
        Action::UserMenu => "user_menu",
        Action::View => "view",
        Action::ViewAlt => "view_alt",
        Action::Edit => "edit",
        Action::Copy => "copy",
        Action::Move => "move",
        Action::MkDir => "mkdir",
        Action::Delete => "delete",
        Action::Menu => "menu",
        Action::Quit => "quit",
        Action::PluginMenu => "plugin_menu",
        Action::InstallDevPlugin => "install_dev_plugin",
        Action::ScreensList => "screens_list",
        Action::NextScreen => "next_screen",
        Action::PrevScreen => "prev_screen",
        Action::PrintFile => "print_file",
        Action::CreateLink => "create_link",
        Action::WipeFile => "wipe_file",
        Action::FileAttributes => "file_attributes",
        Action::ApplyCommand => "apply_command",
        Action::DescribeFile => "describe_file",
        Action::CompressFiles => "compress_files",
        Action::ExtractArchive => "extract_archive",
        Action::ArchiveCommands => "archive_commands",
        Action::SelectGroup => "select_group",
        Action::UnselectGroup => "unselect_group",
        Action::InvertSelection => "invert_selection",
        Action::RestoreSelection => "restore_selection",
        Action::FindFile => "find_file",
        Action::CommandHistory => "command_history",
        Action::FileViewHistory => "file_view_history",
        Action::FoldersHistory => "folders_history",
        Action::CompareFolder => "compare_folder",
        Action::EditUserMenu => "edit_user_menu",
        Action::FileAssociations => "file_associations",
        Action::FolderShortcutsConfig => "folder_shortcuts_config",
        Action::FilePanelFilter => "file_panel_filter",
        Action::QuickFilter => "quick_filter",
        Action::TaskList => "task_list",
        Action::SaveSetup => "save_setup",
        Action::SystemSettings => "system_settings",
        Action::ToggleHidden => "toggle_hidden",
        Action::FocusCli => "focus_cli",
        Action::Unfocus => "unfocus",
        Action::Refresh => "refresh",
        Action::RereadPanel => "reread_panel",
        Action::SwapPanels => "swap_panels",
        Action::DriveSelectLeft => "drive_select_left",
        Action::DriveSelectRight => "drive_select_right",
        Action::ContextMenu => "context_menu",
        Action::GoFolderShortcut(n) => return format!("go_folder_shortcut_{}", n),
        Action::ToggleLongNames => "toggle_long_names",
        Action::VideoMode => "video_mode",
        Action::TreeView => "tree_view",
        Action::CycleFKeysModifiers => "cycle_fkeys_modifiers",
        Action::SshConnect => "ssh_connect",
        Action::SshDisconnect => "ssh_disconnect",
        Action::OpenGitPanel => "open_git_panel",
        Action::ToggleSortReverse => "toggle_sort_reverse",
        Action::CheckForUpdates => "check_for_updates",
        Action::ToggleTransferPanel => "toggle_transfer_panel",
    }
    .to_string()
}

/// Converts a snake_case action name string into an `Action` variant.
/// Used when loading `custom_bindings` from `keybindings.toml`.
pub fn parse_action_name(name: &str) -> Option<Action> {
    // Handle parameterised variants first
    if let Some(rest) = name.strip_prefix("go_folder_shortcut_") {
        if let Ok(n) = rest.parse::<u8>() {
            if (1..=9).contains(&n) {
                return Some(Action::GoFolderShortcut(n));
            }
        }
        return None;
    }

    let name_lower = name.to_lowercase();
    let mut clean_name = name_lower.as_str();

    // Strip known suffixes that allow mapping multiple keys to the same action in TOML
    for suffix in &[
        "_arrow", "_pgkey", "_home", "_end", "_enter", "_bs", "_insert", "_fkey", "_alt", "_shift",
        "_rename", "_f10",
    ] {
        if let Some(stripped) = clean_name.strip_suffix(suffix) {
            clean_name = stripped;
            break;
        }
    }

    if clean_name == "rename" {
        return Some(Action::Move);
    }

    match clean_name {
        // ── Navigation ────────────────────────────────────────────────────────
        "move_up" => Some(Action::MoveUp),
        "move_down" => Some(Action::MoveDown),
        "page_up" => Some(Action::PageUp),
        "page_down" => Some(Action::PageDown),
        "go_to_top" => Some(Action::GoToTop),
        "go_to_bottom" => Some(Action::GoToBottom),
        "change_panel" => Some(Action::ChangePanel),
        "select_item" => Some(Action::SelectItem),
        "execute" => Some(Action::Execute),
        "go_parent" => Some(Action::GoParent),

        // ── Panel view modes ──────────────────────────────────────────────────
        "panel_view_brief" => Some(Action::PanelViewBrief),
        "panel_view_medium" => Some(Action::PanelViewMedium),
        "panel_view_full" => Some(Action::PanelViewFull),
        "panel_view_wide" => Some(Action::PanelViewWide),
        "panel_view_detailed" => Some(Action::PanelViewDetailed),
        "panel_view_descriptions" => Some(Action::PanelViewDescriptions),
        "panel_view_file_owners" => Some(Action::PanelViewFileOwners),
        "panel_view_file_links" => Some(Action::PanelViewFileLinks),
        "panel_view_alt_full" => Some(Action::PanelViewAltFull),

        // ── Panel toggles ─────────────────────────────────────────────────────
        "toggle_panel_left" => Some(Action::TogglePanelLeft),
        "toggle_panel_right" => Some(Action::TogglePanelRight),
        "toggle_both_panels" => Some(Action::ToggleBothPanels),
        "info_panel" => Some(Action::InfoPanel),
        "quick_view" => Some(Action::QuickView),
        "sort_modes" => Some(Action::SortModes),
        "toggle_long_names" => Some(Action::ToggleLongNames),

        // ── F-key actions ─────────────────────────────────────────────────────
        "help" => Some(Action::Help),
        "about" => Some(Action::About),
        "user_menu" => Some(Action::UserMenu),
        "view" => Some(Action::View),
        "view_alt" => Some(Action::ViewAlt),
        "edit" => Some(Action::Edit),
        "copy" => Some(Action::Copy),
        "move" => Some(Action::Move),
        "mkdir" => Some(Action::MkDir),
        "delete" => Some(Action::Delete),
        "menu" => Some(Action::Menu),
        "quit" => Some(Action::Quit),
        "plugin_menu" => Some(Action::PluginMenu),
        "install_dev_plugin" => Some(Action::InstallDevPlugin),
        "screens_list" => Some(Action::ScreensList),
        "next_screen" => Some(Action::NextScreen),
        "prev_screen" => Some(Action::PrevScreen),

        // ── File operations ───────────────────────────────────────────────────
        "print_file" => Some(Action::PrintFile),
        "create_link" => Some(Action::CreateLink),
        "wipe_file" => Some(Action::WipeFile),
        "file_attributes" => Some(Action::FileAttributes),
        "apply_command" => Some(Action::ApplyCommand),
        "describe_file" => Some(Action::DescribeFile),
        "compress_files" => Some(Action::CompressFiles),
        "extract_archive" => Some(Action::ExtractArchive),
        "archive_commands" => Some(Action::ArchiveCommands),

        // ── Bulk selection ────────────────────────────────────────────────────
        "select_group" => Some(Action::SelectGroup),
        "unselect_group" => Some(Action::UnselectGroup),
        "invert_selection" => Some(Action::InvertSelection),
        "restore_selection" => Some(Action::RestoreSelection),

        // ── Search & history ──────────────────────────────────────────────────
        "find_file" => Some(Action::FindFile),
        "command_history" => Some(Action::CommandHistory),
        "file_view_history" => Some(Action::FileViewHistory),
        "folders_history" => Some(Action::FoldersHistory),

        // ── Commands ──────────────────────────────────────────────────────────
        "compare_folder" => Some(Action::CompareFolder),
        "edit_user_menu" => Some(Action::EditUserMenu),
        "file_associations" => Some(Action::FileAssociations),
        "folder_shortcuts_config" => Some(Action::FolderShortcutsConfig),
        "file_panel_filter" => Some(Action::FilePanelFilter),
        "quick_filter" => Some(Action::QuickFilter),
        "task_list" => Some(Action::TaskList),

        // ── Options ───────────────────────────────────────────────────────────
        "save_setup" => Some(Action::SaveSetup),
        "system_settings" => Some(Action::SystemSettings),

        // ── Sorting ───────────────────────────────────────────────────────────
        "sort_by_name" => Some(Action::SortByName),
        "sort_by_extension" => Some(Action::SortByExtension),
        "sort_by_write_time" => Some(Action::SortByWriteTime),
        "sort_by_size" => Some(Action::SortBySize),
        "sort_unsorted" => Some(Action::SortUnsorted),
        "sort_by_creation_time" => Some(Action::SortByCreationTime),
        "sort_by_access_time" => Some(Action::SortByAccessTime),
        "sort_by_description" => Some(Action::SortByDescription),
        "sort_by_owner" => Some(Action::SortByOwner),

        // ── General ───────────────────────────────────────────────────────────
        "toggle_hidden" => Some(Action::ToggleHidden),
        "focus_cli" => Some(Action::FocusCli),
        "unfocus" => Some(Action::Unfocus),
        "refresh" => Some(Action::Refresh),
        "reread_panel" => Some(Action::RereadPanel),
        "swap_panels" => Some(Action::SwapPanels),
        "drive_select_left" => Some(Action::DriveSelectLeft),
        "drive_select_right" => Some(Action::DriveSelectRight),
        "context_menu" => Some(Action::ContextMenu),
        "video_mode" => Some(Action::VideoMode),
        "tree_view" => Some(Action::TreeView),
        "cycle_fkeys_modifiers" => Some(Action::CycleFKeysModifiers),
        "ssh_connect" => Some(Action::SshConnect),
        "ssh_disconnect" => Some(Action::SshDisconnect),
        "open_git_panel" => Some(Action::OpenGitPanel),
        "toggle_sort_reverse" => Some(Action::ToggleSortReverse),
        "check_for_updates" => Some(Action::CheckForUpdates),
        "toggle_transfer_panel" => Some(Action::ToggleTransferPanel),

        _ => None,
    }
}

/// Normalises any user-written key string into the canonical format used by
/// `key_event_to_string`.
/// Examples: "ctrl+w" -> "Ctrl+w", "Ctrl+Shift+w" -> "Ctrl+Shift+W", "shift+w" -> "W".
pub fn normalize_key_string(key: &str) -> String {
    let parts: Vec<&str> = key.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return String::new();
    }

    let mut has_ctrl = false;
    let mut has_alt = false;
    let mut has_shift = false;
    let mut key_code = "";

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" => has_ctrl = true,
            "alt" => has_alt = true,
            "shift" => has_shift = true,
            _ => {
                key_code = part;
            }
        }
    }

    if key_code.is_empty() {
        return String::new();
    }

    let mut code_str = match key_code.to_lowercase().as_str() {
        "space" => "Space".to_string(),
        "up" => "Up".to_string(),
        "down" => "Down".to_string(),
        "left" => "Left".to_string(),
        "right" => "Right".to_string(),
        "tab" => "Tab".to_string(),
        "enter" => "Enter".to_string(),
        "backspace" => "Backspace".to_string(),
        "delete" => "Delete".to_string(),
        "insert" => "Insert".to_string(),
        "esc" | "escape" => "Esc".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" | "pgup" => "PageUp".to_string(),
        "pagedown" | "pgdn" => "PageDown".to_string(),
        _ => {
            let lower = key_code.to_lowercase();
            if lower.starts_with('f') {
                if let Ok(num) = lower[1..].parse::<u32>() {
                    format!("F{}", num)
                } else {
                    key_code.to_string()
                }
            } else {
                key_code.to_string()
            }
        }
    };

    let chars: Vec<char> = key_code.chars().collect();
    if chars.len() == 1 {
        let c = chars[0];
        if c.is_alphabetic() {
            if has_shift {
                code_str = c.to_ascii_uppercase().to_string();
            } else {
                code_str = c.to_ascii_lowercase().to_string();
            }
        }
    }

    let mut canonical_parts = Vec::new();
    if has_ctrl {
        canonical_parts.push("Ctrl");
    }
    if has_alt {
        canonical_parts.push("Alt");
    }
    if has_shift {
        if !(has_ctrl || has_alt) && chars.len() == 1 && chars[0].is_alphabetic() {
            // Shift+char with no other modifiers is canonicalised to just the uppercase char (e.g. "W")
        } else {
            canonical_parts.push("Shift");
        }
    }

    if canonical_parts.is_empty() {
        code_str
    } else {
        format!("{}+{}", canonical_parts.join("+"), code_str)
    }
}
