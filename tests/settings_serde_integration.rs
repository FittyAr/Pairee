//! Settings TOML on disk round-trips through the public `pairee::Settings` type.

use std::fs;

use pairee::{AppState, Settings};

#[test]
fn settings_default_survives_temp_file_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.toml");

    let original = Settings::default();
    let encoded = toml::to_string(&original).expect("serialize settings");
    fs::write(&path, &encoded).expect("write settings");

    let loaded_src = fs::read_to_string(&path).expect("read settings");
    let loaded: Settings = toml::from_str(&loaded_src).expect("deserialize settings");

    assert_eq!(loaded.keybinding_preset, original.keybinding_preset);
    assert_eq!(loaded.theme, original.theme);
    assert_eq!(loaded.language, original.language);
    assert_eq!(loaded.show_hidden, original.show_hidden);
    assert_eq!(loaded.git_enabled, original.git_enabled);
    assert_eq!(loaded.ssh_enabled, original.ssh_enabled);
    assert_eq!(loaded.plugins_enabled, original.plugins_enabled);
    assert_eq!(loaded.image_preview_enabled, original.image_preview_enabled);
    assert_eq!(loaded.onboarding_completed, original.onboarding_completed);
    assert_eq!(loaded.panel_view_mode, original.panel_view_mode);
    assert_eq!(loaded.sort_field, original.sort_field);
}

#[test]
fn settings_file_without_onboarding_field_skips_first_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("settings.toml");

    let mut table: toml::Table =
        toml::from_str(&toml::to_string(&Settings::default()).unwrap()).unwrap();
    table.remove("onboarding_completed");
    fs::write(&path, toml::to_string(&table).unwrap()).expect("write stripped settings");

    let loaded: Settings =
        toml::from_str(&fs::read_to_string(&path).unwrap()).expect("deserialize stripped");
    assert!(
        loaded.onboarding_completed,
        "existing configs without the field must skip onboarding"
    );
}

#[test]
fn app_state_keeps_distinct_panel_roots() {
    let left = tempfile::tempdir().expect("left");
    let right = tempfile::tempdir().expect("right");
    fs::write(left.path().join("only-left.txt"), b"L").unwrap();
    fs::write(right.path().join("only-right.txt"), b"R").unwrap();

    let state = AppState::new(left.path().to_path_buf(), right.path().to_path_buf());
    assert_eq!(state.panels.left.current_path, left.path());
    assert_eq!(state.panels.right.current_path, right.path());
    assert_ne!(
        state.panels.left.current_path,
        state.panels.right.current_path
    );
    assert!(state.panels.left_visible && state.panels.right_visible);
}
