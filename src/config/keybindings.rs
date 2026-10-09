use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Key of [`KeybindingsConfig::overrides`] that applies to every preset.
pub const ALL_PRESETS: &str = "all";

/// User keymap overrides: action id → chords (`"Ctrl+k, F1"`; `""` unbinds).
pub type OverrideTable = BTreeMap<String, String>;

/// Keybinding preset selection and per-user overrides (`keybindings.toml`).
///
/// ## Preset files
/// Each preset is a TOML file in the `keymaps/` subdirectory of the Pairee
/// config folder (e.g. `%APPDATA%\pairee\config\keymaps\` on Windows).
/// Built-in presets: `"norton"` (default), `"standard"`, `"neovim"`, `"yazi"`
/// (`"vscode"` is the old name of `"standard"`).
///
/// ## Custom presets
/// Create `keymaps/<name>.toml` and set `preset = "<name>"`. A preset can
/// inherit another one and only list its differences:
///
/// ```toml
/// extends = "norton"
///
/// [options]
/// typing = "commands"     # cli | type_ahead | commands
/// leader = "Space"        # what <leader> stands for
/// sequence_timeout = 1000 # ms between the keys of a sequence; 0 = wait
///
/// [panels]
/// move_down = "j, Down"
/// go_to_top = "g g, Home"
/// delete    = ""          # unbind what the parent assigned
/// ```
///
/// ## Overrides
/// `[overrides.all]` applies on top of any preset, `[overrides.<preset>]`
/// only on top of that one (and wins over `all`). An override replaces the
/// action's chords and takes a chord away from any action that had it.
///
/// ## Far-style gray keys
/// `Gray+`, `Gray-` and `Gray*` are accepted and mapped to `Plus`, `-` and
/// `*`. Invalid or duplicate chords are listed in Settings → Interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// Active preset profile name. Must match a file in the `keymaps/` directory.
    pub preset: String,
    /// `"all"` or a preset name → overrides.
    #[serde(default)]
    pub overrides: BTreeMap<String, OverrideTable>,
    /// Pre-0.8 flat overrides; read once and moved to `overrides.all`.
    #[serde(default, skip_serializing)]
    custom_bindings: HashMap<String, String>,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            preset: "norton".to_string(),
            overrides: BTreeMap::new(),
            custom_bindings: HashMap::new(),
        }
    }
}

impl KeybindingsConfig {
    /// Parses `keybindings.toml`, moving legacy `custom_bindings` into
    /// `overrides.all` (an existing `all` entry for the same action wins).
    pub fn from_toml(src: &str) -> Result<Self, toml::de::Error> {
        let mut cfg: Self = toml::from_str(src)?;
        cfg.preset = crate::keybindings::embedded::normalize_preset_name(&cfg.preset);
        let legacy = std::mem::take(&mut cfg.custom_bindings);
        if !legacy.is_empty() {
            let all = cfg.overrides.entry(ALL_PRESETS.to_string()).or_default();
            for (action, keys) in legacy {
                all.entry(action).or_insert(keys);
            }
        }
        Ok(cfg)
    }

    /// Override tables that apply to `preset`, lowest priority first.
    pub fn overrides_for<'a>(&'a self, preset: &str) -> Vec<(&'a str, &'a OverrideTable)> {
        [ALL_PRESETS, preset]
            .into_iter()
            .filter_map(|key| self.overrides.get_key_value(key))
            .map(|(k, table)| (k.as_str(), table))
            .collect()
    }

    /// Sets one override (`preset` = `"all"` or a preset name).
    pub fn set_override(&mut self, preset: &str, action: &str, keys: &str) {
        self.overrides
            .entry(preset.to_string())
            .or_default()
            .insert(action.to_string(), keys.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_custom_bindings_move_to_all() {
        let cfg = KeybindingsConfig::from_toml(
            "preset = \"neovim\"\n[custom_bindings]\ncopy = \"F5\"\nquit = \"F10\"\n\
             [overrides.all]\nquit = \"Ctrl+q\"\n",
        )
        .unwrap();
        let all = &cfg.overrides[ALL_PRESETS];
        assert_eq!(all["copy"], "F5");
        assert_eq!(all["quit"], "Ctrl+q");
        let saved = toml::to_string(&cfg).unwrap();
        assert!(!saved.contains("custom_bindings"), "{saved}");
        assert!(saved.contains("[overrides.all]"), "{saved}");
    }

    #[test]
    fn preset_overrides_come_after_all() {
        let mut cfg = KeybindingsConfig::default();
        cfg.set_override("neovim", "copy", "y y");
        cfg.set_override(ALL_PRESETS, "copy", "F5");
        cfg.set_override("norton", "copy", "F15");
        let names: Vec<&str> = cfg
            .overrides_for("neovim")
            .iter()
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(names, ["all", "neovim"]);
    }
}
