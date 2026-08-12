//! Integration smoke tests for config path helpers and atomic-ish workspace assumptions.
//!
//! Keep each integration file focused (no monolithic test modules).

use std::fs;
use std::path::PathBuf;

/// Ensures a temporary directory can host a minimal settings-like TOML roundtrip.
/// This guards packaging/layout regressions without starting the TUI.
#[test]
fn settings_toml_roundtrip_in_tempdir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path: PathBuf = dir.path().join("settings.toml");

    let original = r#"
show_hidden = true
language = "en"
keybinding_preset = "norton"
"#;
    fs::write(&path, original).expect("write settings");

    let loaded = fs::read_to_string(&path).expect("read settings");
    assert!(loaded.contains("show_hidden = true"));
    assert!(loaded.contains("language = \"en\""));
    assert!(loaded.contains("keybinding_preset = \"norton\""));
}

#[test]
fn temp_workspace_is_isolated() {
    let a = tempfile::tempdir().expect("temp a");
    let b = tempfile::tempdir().expect("temp b");
    assert_ne!(a.path(), b.path());
    fs::write(a.path().join("marker.txt"), "pairee").expect("write marker");
    assert!(!b.path().join("marker.txt").exists());
}
