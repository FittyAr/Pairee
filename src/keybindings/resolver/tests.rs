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
    let config = AppConfig::default();
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
    let config = AppConfig::default();
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
    assert_eq!(parse_action_name("git_init"), Some(Action::GitInit));
    assert_eq!(parse_action_name("git_clone"), Some(Action::GitClone));
    assert_eq!(parse_action_name("invalid_action_name"), None);
}

#[test]
fn test_resolve_for_key_string() {
    let config = AppConfig::default();
    let resolver = KeybindingResolver::new(&config);
    assert_eq!(resolver.resolve_for_key_string("F5"), Some(Action::Copy));
    assert_eq!(resolver.resolve_for_key_string("F1"), Some(Action::Help));
}

#[test]
fn custom_which_key_chord_resolves() {
    let mut config = AppConfig::default();
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
    let mut config = AppConfig::default();
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

/// Tab chords every built-in preset binds, as key events.
fn tab_keys() -> Vec<(KeyEvent, Action)> {
    let alt = KeyModifiers::ALT;
    let mut keys = vec![
        (KeyEvent::new(KeyCode::Char('t'), alt), Action::NewTab),
        (KeyEvent::new(KeyCode::Char('w'), alt), Action::CloseTab),
        (KeyEvent::new(KeyCode::PageDown, alt), Action::NextTab),
        (KeyEvent::new(KeyCode::Right, alt), Action::NextTab),
        (KeyEvent::new(KeyCode::PageUp, alt), Action::PrevTab),
        (KeyEvent::new(KeyCode::Left, alt), Action::PrevTab),
        (
            KeyEvent::new(KeyCode::PageUp, alt | KeyModifiers::SHIFT),
            Action::MoveTabLeft,
        ),
        (
            KeyEvent::new(KeyCode::PageDown, alt | KeyModifiers::SHIFT),
            Action::MoveTabRight,
        ),
        (KeyEvent::new(KeyCode::Char('o'), alt), Action::OpenInNewTab),
        (
            KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL),
            Action::OpenInNewTab,
        ),
    ];
    for n in 1..=9u8 {
        let digit = char::from(b'0' + n);
        keys.push((KeyEvent::new(KeyCode::Char(digit), alt), Action::GoToTab(n)));
    }
    keys
}

#[test]
fn every_preset_binds_the_tab_actions_without_conflicts() {
    for preset in ["norton", "neovim", "vscode"] {
        let mut config = AppConfig::default();
        config.keybindings.preset = preset.into();
        let mut resolver = KeybindingResolver::new(&config);
        // Every tab action id contains `_tab`; none may be rejected.
        let errors = &resolver.load_report().errors;
        assert!(
            !errors.iter().any(|e| e.contains("_tab")),
            "{preset}: {errors:?}"
        );
        for (key, action) in tab_keys() {
            assert_eq!(resolver.resolve(key), Some(action), "{preset}: {key:?}");
        }
        let ctrl_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
        let expected = if preset == "norton" {
            Action::ToggleTransferPanel
        } else {
            Action::NewTab
        };
        assert_eq!(resolver.resolve(ctrl_t), Some(expected), "{preset}: Ctrl+T");
    }
}

#[test]
fn tab_action_names_parse() {
    assert_eq!(parse_action_name("go_to_tab_3"), Some(Action::GoToTab(3)));
    assert_eq!(parse_action_name("go_to_tab_0"), None);
    assert_eq!(parse_action_name("new_tab_alt"), Some(Action::NewTab));
    assert_eq!(
        parse_action_name("toggle_tab_lock"),
        Some(Action::ToggleTabLock)
    );
}

#[test]
fn every_preset_binds_folder_sizes_and_disk_usage_on_free_keys() {
    let alt = KeyModifiers::ALT;
    let keys = [
        (KeyCode::Char('s'), Action::CalculateFolderSizes),
        (KeyCode::Char('d'), Action::DiskUsage),
    ];
    for preset in ["norton", "neovim", "vscode"] {
        let mut config = AppConfig::default();
        config.keybindings.preset = preset.into();
        let mut resolver = KeybindingResolver::new(&config);
        // A chord bound twice is rejected with a "Duplicate key chord" error.
        let errors = &resolver.load_report().errors;
        assert!(
            !errors
                .iter()
                .any(|e| ["'Alt+s'", "'Alt+d'", "folder_sizes", "disk_usage"]
                    .iter()
                    .any(|needle| e.contains(needle))),
            "{preset}: {errors:?}"
        );
        for (code, action) in keys {
            let key = KeyEvent::new(code, alt);
            assert_eq!(resolver.resolve(key), Some(action), "{preset}: {key:?}");
        }
    }
}
