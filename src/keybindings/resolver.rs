use super::actions::Action;
use crate::config::AppConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

pub struct KeybindingResolver {
    bindings: HashMap<String, Action>,
}

impl KeybindingResolver {
    pub fn new(config: &AppConfig) -> Self {
        // Load default bindings from selected preset profile
        let mut bindings = super::preset::get_preset_bindings(&config.keybindings.preset);

        // Overlay user custom overrides
        for (action_name, key_str) in &config.keybindings.custom_bindings {
            if let Some(action) = parse_action_name(action_name) {
                bindings.insert(key_str.clone(), action);
            }
        }

        Self { bindings }
    }

    /// Resolves a KeyEvent into a logical Action.
    pub fn resolve(&self, key_event: KeyEvent) -> Option<Action> {
        let key_str = key_event_to_string(key_event);
        if key_str.is_empty() {
            return None;
        }
        self.bindings.get(&key_str).copied()
    }
}

/// Converts a crossterm KeyEvent into a standard human-readable string representation.
/// Examples: "Ctrl+H", "F5", "Alt+F7", "Shift+F9", "Ctrl+Alt+1", "Gray+".
pub fn key_event_to_string(key: KeyEvent) -> String {
    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

    let code_str = match key.code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => {
            // Capitalize if shift is held for cleaner key string representation
            if has_shift && c.is_ascii_lowercase() {
                c.to_ascii_uppercase().to_string()
            } else {
                c.to_string()
            }
        }
        KeyCode::F(num) => format!("F{}", num),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        // Numpad/Gray keys represented by crossterm as Char on some terminals
        _ => String::new(),
    };

    if code_str.is_empty() {
        return String::new();
    }

    // Build modifier prefix
    let mut parts: Vec<&str> = Vec::new();
    if has_ctrl {
        parts.push("Ctrl");
    }
    if has_alt {
        parts.push("Alt");
    }
    if has_shift {
        parts.push("Shift");
    }

    if parts.is_empty() {
        code_str
    } else {
        // Shift+char is already capitalised above — don't add "Shift+" prefix for plain chars
        if parts.len() == 1 && parts[0] == "Shift" && matches!(key.code, KeyCode::Char(_)) {
            return code_str;
        }
        format!("{}+{}", parts.join("+"), code_str)
    }
}

/// Helper to parse keybinding settings action names into Actions.
/// Extended with all new action variants.
fn parse_action_name(name: &str) -> Option<Action> {
    match name.to_lowercase().as_str() {
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

        // ── Panel view modes ─────────────────────────────────────────────────
        "panel_view_brief" => Some(Action::PanelViewBrief),
        "panel_view_medium" => Some(Action::PanelViewMedium),
        "panel_view_full" => Some(Action::PanelViewFull),
        "panel_view_wide" => Some(Action::PanelViewWide),
        "panel_view_detailed" => Some(Action::PanelViewDetailed),
        "panel_view_descriptions" => Some(Action::PanelViewDescriptions),
        "panel_view_file_owners" => Some(Action::PanelViewFileOwners),
        "panel_view_file_links" => Some(Action::PanelViewFileLinks),
        "panel_view_alt_full" => Some(Action::PanelViewAltFull),

        // ── Panel toggles ────────────────────────────────────────────────────
        "toggle_panel_left" => Some(Action::TogglePanelLeft),
        "toggle_panel_right" => Some(Action::TogglePanelRight),
        "toggle_both_panels" => Some(Action::ToggleBothPanels),
        "info_panel" => Some(Action::InfoPanel),
        "quick_view" => Some(Action::QuickView),
        "sort_modes" => Some(Action::SortModes),
        "toggle_long_names" => Some(Action::ToggleLongNames),

        // ── F-key actions ────────────────────────────────────────────────────
        "help" => Some(Action::Help),
        "user_menu" => Some(Action::UserMenu),
        "view" => Some(Action::View),
        "edit" => Some(Action::Edit),
        "copy" => Some(Action::Copy),
        "move" => Some(Action::Move),
        "mkdir" => Some(Action::MkDir),
        "delete" => Some(Action::Delete),
        "menu" => Some(Action::Menu),
        "quit" => Some(Action::Quit),
        "plugin_menu" => Some(Action::PluginMenu),
        "screens_list" => Some(Action::ScreensList),

        // ── File operations ──────────────────────────────────────────────────
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

        // ── Search & history ─────────────────────────────────────────────────
        "find_file" => Some(Action::FindFile),
        "command_history" => Some(Action::CommandHistory),
        "file_view_history" => Some(Action::FileViewHistory),
        "folders_history" => Some(Action::FoldersHistory),

        // ── Commands ─────────────────────────────────────────────────────────
        "compare_folder" => Some(Action::CompareFolder),
        "edit_user_menu" => Some(Action::EditUserMenu),
        "file_associations" => Some(Action::FileAssociations),
        "folder_shortcuts_config" => Some(Action::FolderShortcutsConfig),
        "file_panel_filter" => Some(Action::FilePanelFilter),
        "task_list" => Some(Action::TaskList),

        // ── Options ──────────────────────────────────────────────────────────
        "save_setup" => Some(Action::SaveSetup),
        "system_settings" => Some(Action::SystemSettings),

        // ── General ──────────────────────────────────────────────────────────
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

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::Action;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_key_event_to_string_basic() {
        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(key_event_to_string(key_up), "Up");

        let key_ctrl_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_string(key_ctrl_h), "Ctrl+h");

        let key_shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(key_shift_tab), "Shift+Tab");
    }

    #[test]
    fn test_key_event_to_string_alt_f() {
        let key_alt_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::ALT);
        assert_eq!(key_event_to_string(key_alt_f7), "Alt+F7");

        let key_shift_f9 = KeyEvent::new(KeyCode::F(9), KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(key_shift_f9), "Shift+F9");
    }

    #[test]
    fn test_key_event_to_string_ctrl_alt() {
        let key_ctrl_alt_1 =
            KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL | KeyModifiers::ALT);
        assert_eq!(key_event_to_string(key_ctrl_alt_1), "Ctrl+Alt+1");
    }

    #[test]
    fn test_resolver_norton_standard() {
        let config = AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        };
        let resolver = KeybindingResolver::new(&config);

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_up), Some(Action::MoveUp));

        let key_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_f7), Some(Action::MkDir));

        let key_f8 = KeyEvent::new(KeyCode::F(8), KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_f8), Some(Action::Delete));
    }

    #[test]
    fn test_resolver_new_actions() {
        let config = AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        };
        let resolver = KeybindingResolver::new(&config);

        let key_alt_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::ALT);
        assert_eq!(resolver.resolve(key_alt_f7), Some(Action::FindFile));

        let key_shift_f9 = KeyEvent::new(KeyCode::F(9), KeyModifiers::SHIFT);
        assert_eq!(resolver.resolve(key_shift_f9), Some(Action::SaveSetup));

        let key_ctrl_w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
        assert_eq!(resolver.resolve(key_ctrl_w), Some(Action::TaskList));
    }
}
