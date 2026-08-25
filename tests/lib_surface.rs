//! Smoke: Pairee is a library crate so integration tests can `use pairee`.

use std::path::PathBuf;

use pairee::{AppState, Settings};

#[test]
fn library_exports_app_state() {
    let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
    assert!(!state.terminal_needs_clear);
    assert!(state.needs_redraw());
    state.ui_dirty = false;
    assert!(!state.needs_redraw());
}

#[test]
fn library_exports_settings_defaults() {
    let settings = Settings::default();
    assert!(settings.git_enabled);
    assert!(settings.ssh_enabled);
    assert!(settings.plugins_enabled);
    assert!(settings.image_preview_enabled);
    assert!(!settings.onboarding_completed);
}
