use super::actions::Action;
use super::preset::{normalize_key_string, parse_action_name};
use crate::config::AppConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

pub struct KeybindingResolver {
    bindings: HashMap<String, Action>,
    /// Inverse map: Action -> first key string that triggers it.
    inverse: HashMap<Action, String>,
}

impl KeybindingResolver {
    pub fn new(config: &AppConfig) -> Self {
        // Load bindings from the active preset (file-based or built-in fallback)
        let mut bindings = super::preset::get_preset_bindings(&config.keybindings.preset);

        // Overlay user custom overrides from keybindings.toml [custom_bindings]
        for (action_name, key_str) in &config.keybindings.custom_bindings {
            if let Some(action) = parse_action_name(action_name) {
                for key in key_str.split(',') {
                    let trimmed = key.trim();
                    if !trimmed.is_empty() {
                        bindings.insert(normalize_key_string(trimmed), action);
                    }
                }
            }
        }

        // Build inverse map (action -> first key found, prefer shorter/simpler keys)
        let mut inverse: HashMap<Action, String> = HashMap::new();
        for (key_str, action) in &bindings {
            inverse.entry(*action).or_insert_with(|| key_str.clone());
        }

        Self { bindings, inverse }
    }

    /// Resolves a KeyEvent into a logical Action.
    pub fn resolve(&self, key_event: KeyEvent) -> Option<Action> {
        let key_str = key_event_to_string(key_event);
        if key_str.is_empty() {
            return None;
        }
        self.bindings.get(&key_str).copied()
    }

    /// Returns the key string bound to `action` in the active preset, or `None` if unbound.
    pub fn key_for_action(&self, action: Action) -> Option<&str> {
        self.inverse.get(&action).map(|s| s.as_str())
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
        let key_ctrl_alt_1 = KeyEvent::new(
            KeyCode::Char('1'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
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

        let key_ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert_eq!(
            resolver.resolve(key_ctrl_p),
            Some(Action::CycleFKeysModifiers)
        );
    }

    #[test]
    fn test_action_parsing_with_suffixes() {
        assert_eq!(parse_action_name("move_up_arrow"), Some(Action::MoveUp));
        assert_eq!(parse_action_name("move_down_arrow"), Some(Action::MoveDown));
        assert_eq!(parse_action_name("page_up_pgkey"), Some(Action::PageUp));
        assert_eq!(parse_action_name("page_down_pgkey"), Some(Action::PageDown));
        assert_eq!(parse_action_name("view_fkey"), Some(Action::View));
        assert_eq!(parse_action_name("move_rename"), Some(Action::Move));
        assert_eq!(parse_action_name("rename"), Some(Action::Move));
        assert_eq!(parse_action_name("quit_f10"), Some(Action::Quit));
        assert_eq!(
            parse_action_name("context_menu_shift"),
            Some(Action::ContextMenu)
        );
        assert_eq!(parse_action_name("find_file_alt"), Some(Action::FindFile));
        assert_eq!(parse_action_name("invalid_action_name"), None);
    }
}
