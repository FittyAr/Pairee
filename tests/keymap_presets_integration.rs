//! Shipped keymap files under `keymaps/` must parse and form the documented
//! inheritance: `base` is the root and every selectable preset extends it.
//! What each preset binds is checked through the loader in the unit tests.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    extends: Option<String>,
    #[allow(dead_code)]
    options: Option<toml::Table>,
    panels: BTreeMap<String, String>,
    #[serde(default)]
    editor: BTreeMap<String, String>,
    #[serde(default)]
    viewer: BTreeMap<String, String>,
    #[serde(default)]
    list: BTreeMap<String, String>,
}

fn load_preset(name: &str) -> PresetFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("keymaps")
        .join(format!("{name}.toml"));
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    toml::from_str(&src).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

#[test]
fn base_is_the_root_layer() {
    let base = load_preset("base");
    assert_eq!(base.extends, None);
    assert!(base.panels.len() > 50, "base holds the shared bindings");
    for (section, table) in [
        ("editor", &base.editor),
        ("viewer", &base.viewer),
        ("list", &base.list),
    ] {
        assert!(!table.is_empty(), "base defines the {section} keys");
    }
}

#[test]
fn selectable_presets_extend_base_and_only_list_differences() {
    let base = load_preset("base");
    for name in ["norton", "neovim", "vscode"] {
        let preset = load_preset(name);
        assert_eq!(preset.extends.as_deref(), Some("base"), "{name}");
        let repeated: Vec<&String> = preset
            .panels
            .iter()
            .filter(|(action, keys)| base.panels.get(*action) == Some(keys))
            .map(|(action, _)| action)
            .collect();
        assert!(repeated.is_empty(), "{name} repeats base: {repeated:?}");
    }
}
