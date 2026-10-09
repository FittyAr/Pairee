use super::*;
use crate::keybindings::preset::parse_id;

fn parse_action_name(name: &str) -> Option<Action> {
    parse_id::<Action>(name).map(|p| p.command)
}
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

    // Norton Commander: F7 makes a folder, Shift+F6 renames.
    let key_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::empty());
    assert_eq!(resolver.resolve(key_f7), Some(Action::MkDir));
    let key_shift_f6 = KeyEvent::new(KeyCode::F(6), KeyModifiers::SHIFT);
    assert_eq!(resolver.resolve(key_shift_f6), Some(Action::Rename));

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

    // Far: Ctrl+P hides the passive panel.
    let key_ctrl_p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert_eq!(
        resolver.resolve(key_ctrl_p),
        Some(Action::ToggleInactivePanel)
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
        .set_override("all", "which_key", "Ctrl+Alt+Shift+F11");
    let resolver = KeybindingResolver::new(&config);
    assert_eq!(
        resolver.resolve_for_key_string("Ctrl+Alt+Shift+F11"),
        Some(Action::WhichKey)
    );
}

#[test]
fn prefix_completions_list_remaining_suffixes() {
    let mut config = AppConfig::default();
    config.keybindings.set_override("all", "about", "Alt+q x");
    config.keybindings.set_override("all", "help", "Alt+q h");
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

#[test]
fn every_preset_reaches_tabs_folder_sizes_and_disk_usage() {
    use crate::keybindings::embedded::PRESETS;
    let mut wanted = vec![
        Action::NewTab,
        Action::CloseTab,
        Action::NextTab,
        Action::PrevTab,
        Action::MoveTabLeft,
        Action::MoveTabRight,
        Action::OpenInNewTab,
        Action::CalculateFolderSizes,
        Action::DiskUsage,
    ];
    wanted.extend((1..=9).map(Action::GoToTab));
    for (preset, _) in PRESETS {
        let mut config = AppConfig::default();
        config.keybindings.preset = (*preset).into();
        let resolver = KeybindingResolver::new(&config);
        assert!(resolver.load_report().ok(), "{preset}");
        for action in &wanted {
            assert!(
                resolver.key_for_action(*action).is_some(),
                "{preset}: {action:?} has no key"
            );
        }
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
fn a_stale_sequence_expires_on_tick() {
    let mut config = AppConfig::default();
    config.keybindings.set_override("all", "about", "Alt+q x");
    let mut resolver = KeybindingResolver::new(&config);
    let start = std::time::Instant::now();
    assert!(!resolver.expire_pending(start));
    resolver.resolve(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT));
    assert!(resolver.is_ongoing());
    assert!(!resolver.expire_pending(std::time::Instant::now()));
    let late = std::time::Instant::now() + std::time::Duration::from_secs(2);
    assert!(resolver.expire_pending(late));
    assert!(!resolver.is_ongoing());
}

#[test]
fn ctrl_shift_tab_reaches_prev_screen_over_the_screens() {
    let resolver = KeybindingResolver::new(&AppConfig::default());
    let back_tab = KeyEvent::new(
        KeyCode::BackTab,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(resolver.global_action(back_tab), Some(Action::PrevScreen));
    let f12 = KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE);
    assert_eq!(resolver.global_action(f12), Some(Action::ScreensList));
    let f5 = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
    assert_eq!(resolver.global_action(f5), None, "copy is not global");
}
