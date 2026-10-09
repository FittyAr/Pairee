//! Commands a plugin declares in `manifest.toml`, and the keys it suggests:
//!
//! ```toml
//! [[commands]]
//! id    = "toggle"          # keymap id: plugin.<name>.toggle
//! title = "Toggle blame"
//! [commands.keys]           # per preset; "default" for the rest
//! default = "Alt+Shift+B"
//! neovim  = "<leader>gb"
//! ```
//!
//! The pre-v2 `[keybindings] "key" = "action"` table still works: each
//! action becomes a command with those keys as its default.

use super::keyspec::canonical_key;
use crate::keybindings::plugin_commands::PluginCommand;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

/// One `[[commands]]` entry.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommandDecl {
    pub id: String,
    pub title: Option<String>,
    /// Preset name or `default` → chords.
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
}

/// The commands of plugin `plugin`, with keys in keymap grammar.
pub fn plugin_commands(
    plugin: &str,
    declared: &[CommandDecl],
    legacy: Option<&HashMap<String, String>>,
) -> Vec<PluginCommand> {
    let mut out: Vec<PluginCommand> = declared
        .iter()
        .map(|decl| PluginCommand {
            plugin: plugin.to_string(),
            command: decl.id.clone(),
            title: decl.title.clone().unwrap_or_else(|| decl.id.clone()),
            keys: decl
                .keys
                .iter()
                .map(|(preset, chords)| (preset.clone(), normalize_chords(chords)))
                .collect(),
        })
        .collect();
    let mut legacy_keys: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (key, action) in legacy.into_iter().flatten() {
        legacy_keys
            .entry(action)
            .or_default()
            .push(normalize_chords(key));
    }
    for (action, mut keys) in legacy_keys {
        if out.iter().any(|c| c.command == action) {
            continue;
        }
        keys.sort();
        out.push(PluginCommand {
            plugin: plugin.to_string(),
            command: action.to_string(),
            title: action.to_string(),
            keys: BTreeMap::from([("default".to_string(), keys.join(", "))]),
        });
    }
    out
}

/// Rewrites each chord written in a plugin spelling (`ctrl-h`, `<C-h>`)
/// in keymap grammar; sequences and `<leader>` chords pass unchanged.
fn normalize_chords(chords: &str) -> String {
    chords
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|c| canonical_key(c).unwrap_or_else(|| c.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_and_legacy_commands_become_keymap_commands() {
        let declared = vec![CommandDecl {
            id: "toggle".into(),
            title: Some("Toggle blame".into()),
            keys: BTreeMap::from([
                ("default".into(), "ctrl-b, <A-x>".into()),
                ("neovim".into(), "<leader>gb".into()),
            ]),
        }];
        let legacy = HashMap::from([
            ("ctrl+h".to_string(), "hello".to_string()),
            ("ctrl-j".to_string(), "hello".to_string()),
            ("ctrl+t".to_string(), "toggle".to_string()),
        ]);
        let cmds = plugin_commands("blame", &declared, Some(&legacy));
        assert_eq!(cmds.len(), 2, "a declared command wins over a legacy one");
        assert_eq!(cmds[0].keymap_id(), "plugin.blame.toggle");
        assert_eq!(cmds[0].keys_for("norton"), Some("Ctrl+b, Alt+x"));
        assert_eq!(cmds[0].keys_for("neovim"), Some("<leader>gb"));
        assert_eq!(cmds[1].title, "hello");
        assert_eq!(cmds[1].keys_for("yazi"), Some("Ctrl+h, Ctrl+j"));
    }
}
