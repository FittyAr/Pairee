use super::*;
use crate::config::keybindings::ALL_PRESETS;
use crate::keybindings::options::TypingMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) const BUILTIN: [&str; 4] = ["norton", "standard", "neovim", "yazi"];

pub(super) fn embedded(name: &str, _: &mut KeymapLoadReport) -> Option<String> {
    embedded::preset_toml(name).map(str::to_string)
}

/// The shipped `preset` plus `overrides` as `(table, action, chords)`.
pub(super) fn load(preset: &str, overrides: &[(&str, &str, &str)]) -> LoadedKeymap {
    load_from(&embedded, preset, overrides)
}

fn load_from(source: PresetSource, preset: &str, overrides: &[(&str, &str, &str)]) -> LoadedKeymap {
    let mut keybindings = KeybindingsConfig::default();
    for (table, action, keys) in overrides {
        keybindings.set_override(table, action, keys);
    }
    build_keymap(
        &KeymapSpec {
            preset,
            keybindings: &keybindings,
            yazi_letters: false,
        },
        source,
    )
}

fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

fn dispatch(map: &mut LoadedKeymap, event: KeyEvent) -> Option<Action> {
    map.panels.dispatch(event)
}

fn chords_of(map: &LoadedKeymap, action: Action) -> Vec<String> {
    map.panels
        .rows()
        .iter()
        .filter(|r| r.command == action)
        .map(|r| r.seq.to_string())
        .collect()
}

#[test]
fn shipped_presets_load_without_errors_or_warnings() {
    for preset in BUILTIN {
        let map = load(preset, &[]);
        let report = &map.report;
        assert!(
            report.errors.is_empty() && report.warnings.is_empty() && report.robustness.is_empty(),
            "{preset}: {:?} {:?} {:?}",
            report.errors,
            report.warnings,
            report.robustness
        );
        assert!(map.report.bound_count > 100, "{preset}");
    }
}

#[test]
fn norton_loads_core_bindings() {
    let mut map = load("norton", &[]);
    assert_eq!(
        dispatch(&mut map, key(KeyCode::Up, KeyModifiers::NONE)),
        Some(Action::MoveUp)
    );
    assert_eq!(
        dispatch(&mut map, key(KeyCode::F(5), KeyModifiers::NONE)),
        Some(Action::Copy)
    );
    assert_eq!(
        dispatch(&mut map, key(KeyCode::F(3), KeyModifiers::ALT)),
        Some(Action::ViewAlt)
    );
}

#[test]
fn base_layer_reaches_every_preset() {
    for preset in BUILTIN {
        let mut map = load(preset, &[]);
        let help = key(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(dispatch(&mut map, help), Some(Action::Help), "{preset}");
        let shortcut = key(
            KeyCode::Char('3'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_eq!(
            dispatch(&mut map, shortcut),
            Some(Action::GoFolderShortcut(3)),
            "{preset}"
        );
        assert!(
            map.panels
                .rows()
                .iter()
                .any(|r| r.origin == Origin::Preset("base".into()))
        );
    }
}

#[test]
fn an_override_replaces_chords_and_takes_them_from_their_owner() {
    let mut map = load("norton", &[(ALL_PRESETS, "delete", "F5")]);
    assert!(map.report.errors.is_empty(), "{:?}", map.report.errors);
    assert_eq!(chords_of(&map, Action::Delete), ["F5"]);
    assert!(chords_of(&map, Action::Copy).is_empty());
    assert!(
        map.report
            .displaced
            .iter()
            .any(|d| d.contains("copy → delete"))
    );
    assert_eq!(
        dispatch(&mut map, key(KeyCode::F(8), KeyModifiers::NONE)),
        None
    );
}

#[test]
fn an_empty_override_unbinds() {
    let map = load("norton", &[(ALL_PRESETS, "copy", "")]);
    assert!(chords_of(&map, Action::Copy).is_empty());
    assert!(map.report.errors.is_empty());
}

#[test]
fn preset_overrides_win_over_all_and_only_apply_to_their_preset() {
    let overrides = [(ALL_PRESETS, "copy", "F15"), ("neovim", "copy", "F16")];
    assert_eq!(
        chords_of(&load("neovim", &overrides), Action::Copy),
        ["F16"]
    );
    assert_eq!(
        chords_of(&load("norton", &overrides), Action::Copy),
        ["F15"]
    );
}

#[test]
fn two_actions_claiming_a_chord_in_one_layer_is_an_error() {
    let src = |name: &str, _: &mut KeymapLoadReport| {
        (name == "dup").then(|| "[panels]\ncopy = \"F5\"\nmove = \"F5\"".to_string())
    };
    let map = load_from(&src, "dup", &[]);
    assert!(
        map.report.errors.iter().any(|e| e.contains("Duplicate")),
        "{:?}",
        map.report.errors
    );
}

#[test]
fn loading_is_deterministic() {
    let render = |map: LoadedKeymap| {
        map.panels
            .rows()
            .iter()
            .map(|r| format!("{} {:?} {}", r.seq, r.command, r.origin))
            .collect::<Vec<_>>()
    };
    for preset in BUILTIN {
        let first = render(load(preset, &[(ALL_PRESETS, "delete", "F5")]));
        for _ in 0..20 {
            assert_eq!(
                render(load(preset, &[(ALL_PRESETS, "delete", "F5")])),
                first
            );
        }
    }
}

#[test]
fn a_chord_hiding_a_sequence_is_an_error() {
    let map = load(
        "norton",
        &[
            (ALL_PRESETS, "about", "Alt+q"),
            (ALL_PRESETS, "help", "Alt+q h"),
        ],
    );
    assert!(
        map.report
            .errors
            .iter()
            .any(|e| e.contains("hides the sequence")),
        "{:?}",
        map.report.errors
    );
}

#[test]
fn custom_preset_with_options_leader_and_legacy_alias() {
    let src = |name: &str, report: &mut KeymapLoadReport| match name {
        "mine" => Some(
            "extends = \"norton\"\n[options]\ntyping = \"commands\"\nleader = \"Space\"\n\
             sequence_timeout = 0\n[panels]\nfind_file = \"<leader>ff\"\nmove_up_arrow = \"k\"\n\
             select_item = \"Insert\""
                .to_string(),
        ),
        other => embedded(other, report),
    };
    let mut map = load_from(&src, "mine", &[]);
    assert_eq!(map.options.typing, TypingMode::Commands);
    assert_eq!(chords_of(&map, Action::FindFile), ["Space f f"]);
    assert_eq!(chords_of(&map, Action::MoveUp), ["k"]);
    assert!(
        map.report
            .warnings
            .iter()
            .any(|w| w.contains("deprecated alias"))
    );
    let f = key(KeyCode::Char('f'), KeyModifiers::NONE);
    assert_eq!(
        dispatch(&mut map, key(KeyCode::Char(' '), KeyModifiers::NONE)),
        None
    );
    assert_eq!(dispatch(&mut map, f), None);
    assert_eq!(dispatch(&mut map, f), Some(Action::FindFile));
}

#[test]
fn an_older_copy_of_a_builtin_gets_new_actions_on_free_chords() {
    let src = |name: &str, report: &mut KeymapLoadReport| match name {
        "norton" => Some("[bindings]\ncopy = \"F5\"\nmove = \"Ctrl+\\\\\"".to_string()),
        other => embedded(other, report),
    };
    let map = load_from(&src, "norton", &[]);
    assert_eq!(chords_of(&map, Action::Delete), ["F8"]);
    assert!(
        chords_of(&map, Action::GoRoot).is_empty(),
        "Ctrl+\\ is taken by move"
    );
}

#[test]
fn an_older_copy_with_single_letters_gets_no_hidden_sequences() {
    let src = |name: &str, report: &mut KeymapLoadReport| match name {
        "neovim" => Some(
            "[bindings]
go_to_top = \"g\"
copy = \"y\""
                .to_string(),
        ),
        other => embedded(other, report),
    };
    let map = load_from(&src, "neovim", &[]);
    assert!(map.report.errors.is_empty(), "{:?}", map.report.errors);
    assert!(
        chords_of(&map, Action::GoHome).is_empty(),
        "`g h` would hide behind `g`"
    );
    assert!(
        chords_of(&map, Action::Yank).is_empty(),
        "`y y` would hide behind `y`"
    );
    assert_eq!(
        chords_of(&map, Action::Delete),
        ["F8"],
        "free chords still fill in"
    );
}

#[test]
fn yazi_setting_binds_its_letters() {
    let keybindings = KeybindingsConfig::default();
    let spec = KeymapSpec {
        preset: "norton",
        keybindings: &keybindings,
        yazi_letters: true,
    };
    let map = build_keymap(&spec, &embedded);
    assert_eq!(chords_of(&map, Action::SortMenu), ["s"]);
    assert_eq!(chords_of(&map, Action::ViewModeMenu), ["v"]);
}

#[test]
fn unknown_preset_falls_back_to_norton() {
    let map = load("no-such-preset", &[]);
    assert!(
        map.report
            .warnings
            .iter()
            .any(|w| w.contains("falling back"))
    );
    assert_eq!(chords_of(&map, Action::Copy), ["F5"]);
}

#[test]
fn invalid_chords_are_reported_with_their_text() {
    let map = load("norton", &[(ALL_PRESETS, "copy", "Ctrl+rj")]);
    let details = map.report.detail_lines().join("\n");
    assert!(details.contains("Ctrl+rj"), "{details}");
    assert!(!map.report.ok());
}

#[test]
fn friendly_aliases_are_accepted() {
    let map = load(
        "norton",
        &[
            (ALL_PRESETS, "select_group", "Gray+"),
            (ALL_PRESETS, "which_key", "Ctrl+Shift+k"),
            (ALL_PRESETS, "system_settings", "Ctrl+Comma"),
        ],
    );
    assert!(map.report.errors.is_empty(), "{:?}", map.report.errors);
    assert_eq!(chords_of(&map, Action::SelectGroup), ["Plus"]);
    assert_eq!(chords_of(&map, Action::WhichKey), ["Ctrl+K"]);
    assert_eq!(chords_of(&map, Action::SystemSettings), ["Ctrl+,"]);
}

#[test]
fn unbinding_an_essential_action_is_a_robustness_note() {
    let map = load("norton", &[(ALL_PRESETS, "quit", "")]);
    assert!(
        map.report
            .robustness
            .iter()
            .any(|r| r.contains("'quit' has no key"))
    );
    assert!(map.report.ok());
}
