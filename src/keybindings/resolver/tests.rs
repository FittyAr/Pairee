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
    assert_eq!(parse_action_name("copy_path"), Some(Action::CopyPath));
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

#[test]
fn custom_which_key_chord_resolves() {
    let mut config = AppConfig {
        settings: crate::config::settings::Settings::default(),
        theme: crate::config::theme::Theme::default(),
        keybindings: crate::config::keybindings::KeybindingsConfig::default(),
    };
    config
        .keybindings
        .custom_bindings
        .insert("which_key".into(), "Ctrl+Alt+Shift+F11".into());
    let resolver = KeybindingResolver::new(&config);
    assert_eq!(
        resolver.resolve_for_key_string("Ctrl+Alt+Shift+F11"),
        Some(Action::WhichKey)
    );
}

#[test]
fn prefix_completions_list_remaining_suffixes() {
    let mut config = AppConfig {
        settings: crate::config::settings::Settings::default(),
        theme: crate::config::theme::Theme::default(),
        keybindings: crate::config::keybindings::KeybindingsConfig::default(),
    };
    config
        .keybindings
        .custom_bindings
        .insert("about".into(), "Alt+q x".into());
    config
        .keybindings
        .custom_bindings
        .insert("help".into(), "Alt+q h".into());
    let mut resolver = KeybindingResolver::new(&config);

    let alt_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT);
    assert_eq!(resolver.resolve(alt_q), None);
    assert!(resolver.is_ongoing());
    assert!(
        resolver
            .ongoing_prefix_display()
            .to_lowercase()
            .contains('q'),
        "prefix display should mention q, got {}",
        resolver.ongoing_prefix_display()
    );

    let comps = resolver.prefix_completions();
    assert!(
        comps.iter().any(|(_, _, a)| *a == Action::About),
        "expected about in {comps:?}"
    );
    assert!(
        comps.iter().any(|(_, _, a)| *a == Action::Help),
        "expected help in {comps:?}"
    );

    resolver.reset();
    assert!(!resolver.is_ongoing());
    assert!(resolver.prefix_completions().is_empty());
}
