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

/// `(preset, parent)`: Far and standard build on base; the Vim presets on
/// the shared `vi` layer, which builds on Far.
const TREE: [(&str, &str); 5] = [
    ("norton", "base"),
    ("standard", "base"),
    ("vi", "norton"),
    ("neovim", "vi"),
    ("yazi", "vi"),
];

#[test]
fn presets_extend_their_parent_and_only_list_differences() {
    for (name, parent) in TREE {
        let preset = load_preset(name);
        assert_eq!(preset.extends.as_deref(), Some(parent), "{name}");
        let parent = load_preset(parent);
        let tables = [
            (&preset.panels, &parent.panels),
            (&preset.editor, &parent.editor),
            (&preset.viewer, &parent.viewer),
            (&preset.list, &parent.list),
        ];
        for (own, inherited) in tables {
            let repeated: Vec<&String> = own
                .iter()
                .filter(|(action, keys)| inherited.get(*action) == Some(keys))
                .map(|(action, _)| action)
                .collect();
            assert!(
                repeated.is_empty(),
                "{name} repeats its parent: {repeated:?}"
            );
        }
    }
}
