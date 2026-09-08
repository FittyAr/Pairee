//! Shipped keymap files under `keymaps/` must parse and keep distinct presets.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PresetFile {
    bindings: HashMap<String, String>,
}

fn load_preset(name: &str) -> PresetFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("keymaps")
        .join(name);
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    toml::from_str(&src).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

#[test]
fn norton_keymap_uses_arrow_and_f_keys() {
    let preset = load_preset("norton.toml");
    assert_eq!(
        preset.bindings.get("move_up").map(String::as_str),
        Some("Up")
    );
    assert_eq!(
        preset.bindings.get("move_down").map(String::as_str),
        Some("Down")
    );
    assert!(
        preset
            .bindings
            .get("copy")
            .is_some_and(|keys| keys.contains("F5")),
        "Norton copy should include F5, got {:?}",
        preset.bindings.get("copy")
    );
    assert!(
        !preset.bindings.values().any(|v| v.trim().is_empty()),
        "Norton chords must not be empty"
    );
    assert_eq!(
        preset.bindings.get("copy_path").map(String::as_str),
        Some("Ctrl+Shift+c")
    );
}

#[test]
fn neovim_keymap_uses_hjkl_for_navigation() {
    let preset = load_preset("neovim.toml");
    assert_eq!(
        preset.bindings.get("move_up").map(String::as_str),
        Some("k")
    );
    assert_eq!(
        preset.bindings.get("move_down").map(String::as_str),
        Some("j")
    );
    assert_eq!(
        preset.bindings.get("go_parent").map(String::as_str),
        Some("h")
    );
    assert_eq!(
        preset.bindings.get("execute").map(String::as_str),
        Some("l")
    );
    assert_eq!(
        preset.bindings.get("copy_path").map(String::as_str),
        Some("Ctrl+Shift+c")
    );
}

#[test]
fn vscode_keymap_uses_ctrl_c_for_copy() {
    let preset = load_preset("vscode.toml");
    assert_eq!(
        preset.bindings.get("copy").map(String::as_str),
        Some("Ctrl+c")
    );
    assert_eq!(
        preset.bindings.get("move").map(String::as_str),
        Some("Ctrl+x")
    );
    assert!(preset.bindings.contains_key("select_item"));
    assert_eq!(
        preset.bindings.get("copy_path").map(String::as_str),
        Some("Ctrl+Shift+c")
    );
}
