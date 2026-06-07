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

/// Converts a crossterm KeyEvent into a standard human-readable string representation (e.g. "Ctrl+H", "F5").
pub fn key_event_to_string(key: KeyEvent) -> String {
    let mut parts = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }

    let code_str = match key.code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => {
            // Capitalize if shift is held for cleaner key string representation
            if key.modifiers.contains(KeyModifiers::SHIFT) && c.is_ascii_lowercase() {
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
        _ => "".to_string(),
    };

    if code_str.is_empty() {
        return "".to_string();
    }

    if parts.is_empty() {
        code_str
    } else {
        if parts.len() == 1 && parts[0] == "Shift" && matches!(key.code, KeyCode::Char(_)) {
            return code_str;
        }
        format!("{}+{}", parts.join("+"), code_str)
    }
}

/// Helper to parse keybinding settings action names into Actions.
fn parse_action_name(name: &str) -> Option<Action> {
    match name.to_lowercase().as_str() {
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
        "toggle_hidden" => Some(Action::ToggleHidden),
        "focus_cli" => Some(Action::FocusCli),
        "unfocus" => Some(Action::Unfocus),
        "refresh" => Some(Action::Refresh),
        "swap_panels" => Some(Action::SwapPanels),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::Action;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_key_event_to_string() {
        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(key_event_to_string(key_up), "Up");

        let key_ctrl_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
        assert_eq!(key_event_to_string(key_ctrl_h), "Ctrl+h");

        let key_shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(key_event_to_string(key_shift_tab), "Shift+Tab");
    }

    #[test]
    fn test_resolver_norton() {
        let config = AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        };
        let resolver = KeybindingResolver::new(&config);

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_up), Some(Action::MoveUp));

        let key_f5 = KeyEvent::new(KeyCode::F(5), KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_f5), Some(Action::Copy));
    }
}
