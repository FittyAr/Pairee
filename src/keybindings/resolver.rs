//! Keybinding resolver backed by the industry `keybinds` crate.
//!
//! Crossterm still delivers raw `KeyEvent`s; **mapping** is owned by `keybinds`
//! (parse + dispatch + sequences). Invalid chords never enter the map.

use super::actions::Action;
use super::loader::{KeymapLoadReport, load_keybinds};
use crate::config::AppConfig;
use crossterm::event::{KeyEvent, KeyEventKind};
use keybinds::{KeyInput, KeySeq, Keybinds, Match};
use std::collections::HashMap;

pub struct KeybindingResolver {
    keybinds: Keybinds<Action>,
    /// Action → first bound chord display (for F-key bar / help).
    inverse: HashMap<Action, String>,
    #[allow(dead_code)]
    load_report: KeymapLoadReport,
}

impl KeybindingResolver {
    pub fn new(config: &AppConfig) -> Self {
        let (keybinds, report) = load_keybinds(
            &config.keybindings.preset,
            &config.keybindings.custom_bindings,
        );

        for w in &report.warnings {
            log::warn!("keymap: {w}");
        }
        for e in &report.errors {
            log::error!("keymap: {e}");
        }
        if !report.ok() {
            log::error!(
                "keymap loaded with {} error(s); {} binding(s) active",
                report.errors.len(),
                report.bound_count
            );
        } else {
            log::info!(
                "keymap preset='{}' loaded ({} bindings)",
                config.keybindings.preset,
                report.bound_count
            );
        }

        let mut inverse: HashMap<Action, String> = HashMap::new();
        for bind in keybinds.as_slice() {
            inverse
                .entry(bind.action)
                .or_insert_with(|| bind.seq.to_string());
        }

        Self {
            keybinds,
            inverse,
            load_report: report,
        }
    }

    /// Resolve a key press into an action (may complete a multi-key sequence).
    pub fn resolve(&mut self, key_event: KeyEvent) -> Option<Action> {
        // Ignore key-release / non-press noise from enhancement flags.
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return None;
        }
        self.keybinds.dispatch(key_event).copied()
    }

    /// True if this key is a complete binding or starts a multi-key sequence.
    /// Used to keep CLI capture from eating shortcuts (immutable, no dispatch).
    pub fn would_trigger(&self, key_event: KeyEvent) -> bool {
        if key_event.kind != KeyEventKind::Press && key_event.kind != KeyEventKind::Repeat {
            return false;
        }
        if self.keybinds.is_ongoing() {
            return true;
        }
        let input = KeyInput::from(&key_event);
        let single = [input];
        for bind in self.keybinds.as_slice() {
            match bind.seq.match_to(&single) {
                Match::Matched | Match::Prefix => return true,
                Match::Unmatch => {}
            }
        }
        false
    }

    /// Returns the key string bound to `action`, or `None` if unbound.
    pub fn key_for_action(&self, action: Action) -> Option<&str> {
        self.inverse.get(&action).map(|s| s.as_str())
    }

    /// Resolve a config-style key string (e.g. `"F7"`, `"Alt+F5"`) to its action.
    pub fn resolve_for_key_string(&self, key: &str) -> Option<Action> {
        let seq: KeySeq = key.parse().ok()?;
        self.keybinds
            .as_slice()
            .iter()
            .find(|b| b.seq == seq)
            .map(|b| b.action)
    }
}

/// Human-readable key for plugins / logging (best-effort; not the source of truth).
pub fn key_event_to_string(key: KeyEvent) -> String {
    let input = KeyInput::from(&key);
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::preset::parse_action_name;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_key_event_to_string_basic() {
        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(key_event_to_string(key_up), "Up");

        let key_ctrl_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
        let s = key_event_to_string(key_ctrl_h);
        assert!(s.contains("Ctrl") && s.to_lowercase().contains('h'), "{s}");
    }

    #[test]
    fn test_resolver_norton_standard() {
        let config = AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        };
        let mut resolver = KeybindingResolver::new(&config);

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_up), Some(Action::MoveUp));

        let key_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::empty());
        assert_eq!(resolver.resolve(key_f7), Some(Action::Rename));

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
        let mut resolver = KeybindingResolver::new(&config);

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
        assert_eq!(parse_action_name("rename"), Some(Action::Rename));
        assert_eq!(parse_action_name("quit_f10"), Some(Action::Quit));
        assert_eq!(
            parse_action_name("context_menu_shift"),
            Some(Action::ContextMenu)
        );
        assert_eq!(parse_action_name("find_file_alt"), Some(Action::FindFile));
        assert_eq!(parse_action_name("invalid_action_name"), None);
    }

    #[test]
    fn test_resolve_for_key_string() {
        let config = AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        };
        let resolver = KeybindingResolver::new(&config);
        assert_eq!(resolver.resolve_for_key_string("F5"), Some(Action::Copy));
        assert_eq!(resolver.resolve_for_key_string("F1"), Some(Action::Help));
    }
}
