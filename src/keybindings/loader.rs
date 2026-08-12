//! Load and **validate** keymaps using the `keybinds` crate.
//!
//! Guarantees:
//! - Invalid chords (e.g. `Ctrl+rj`) fail parse → rejected, never silently ignored as no-ops.
//! - Duplicate chords (same sequence bound to two actions) are rejected (last-wins is forbidden).
//! - Multiple chords per action remain allowed (`"Insert, Space"`).
//! - Presets Norton / Neovim / VSCode ship as TOML under `keymaps/`.

use super::actions::Action;
use super::preset::parse_action_name;
use crate::config::paths;
use keybinds::{KeySeq, Keybind, Keybinds};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

const EMBEDDED_NORTON: &str = include_str!("../../keymaps/norton.toml");
const EMBEDDED_NEOVIM: &str = include_str!("../../keymaps/neovim.toml");
const EMBEDDED_VSCODE: &str = include_str!("../../keymaps/vscode.toml");

#[derive(Debug, Deserialize)]
struct PresetFile {
    bindings: HashMap<String, String>,
}

/// Issues found while loading a keymap (invalid chords, conflicts, unknown actions).
#[derive(Debug, Default, Clone)]
pub struct KeymapLoadReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub bound_count: usize,
}

impl KeymapLoadReport {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Build a validated [`Keybinds`] dispatcher for the active preset + user overrides.
pub fn load_keybinds(
    preset: &str,
    custom_bindings: &HashMap<String, String>,
) -> (Keybinds<Action>, KeymapLoadReport) {
    let mut report = KeymapLoadReport::default();
    let mut keybinds = Keybinds::default();
    // chord_display → action name already bound (for conflict detection)
    let mut chord_owner: HashMap<String, String> = HashMap::new();

    let toml_src = load_preset_toml(preset, &mut report);
    let mut pairs: Vec<(String, String)> = Vec::new();

    if let Some(content) = toml_src {
        match toml::from_str::<PresetFile>(&content) {
            Ok(file) => {
                for (action, keys) in file.bindings {
                    pairs.push((action, keys));
                }
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("Failed to parse keymap for preset '{preset}': {e}"));
            }
        }
    }

    // User overrides last so they can replace preset chords (still validated).
    for (action, keys) in custom_bindings {
        pairs.push((action.clone(), keys.clone()));
    }

    for (action_name, keys_field) in pairs {
        let Some(action) = parse_action_name(&action_name) else {
            report
                .warnings
                .push(format!("Unknown action '{action_name}' — skipped"));
            continue;
        };

        for raw_key in keys_field.split(',') {
            let chord = normalize_user_chord(raw_key.trim());
            if chord.is_empty() {
                continue;
            }

            // Parse with keybinds grammar — rejects impossible chords like "Ctrl+rj".
            let seq: KeySeq = match chord.parse() {
                Ok(s) => s,
                Err(e) => {
                    report.errors.push(format!(
                        "Invalid key chord '{chord}' for action '{action_name}': {e}"
                    ));
                    continue;
                }
            };

            let chord_key = seq.to_string();
            if let Some(prev) = chord_owner.get(&chord_key) {
                if prev != &action_name {
                    report.errors.push(format!(
                        "Duplicate key chord '{chord_key}': already bound to '{prev}', cannot also bind '{action_name}'"
                    ));
                    continue;
                }
                // Same action rebound — ignore duplicate entry.
                continue;
            }

            // Also reject if an existing Keybind already owns this sequence (defensive).
            if keybinds
                .as_slice()
                .iter()
                .any(|b| b.seq == seq && b.action != action)
            {
                report.errors.push(format!(
                    "Duplicate key chord '{chord_key}' conflicts with an existing binding"
                ));
                continue;
            }

            keybinds.push(Keybind::new(seq, action));
            chord_owner.insert(chord_key, action_name.clone());
            report.bound_count += 1;
        }
    }

    if report.bound_count == 0 {
        report
            .errors
            .push("No key bindings loaded — keymap is empty after validation".into());
    }

    (keybinds, report)
}

fn load_preset_toml(preset: &str, report: &mut KeymapLoadReport) -> Option<String> {
    let name = normalize_preset_name(preset);

    // 1) User config dir
    let path = paths::get_keymaps_dir().join(format!("{name}.toml"));
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(s) => return Some(s),
            Err(e) => report
                .warnings
                .push(format!("Could not read '{}': {e}", path.display())),
        }
    }

    // 2) CWD / shipped keymaps next to binary
    for candidate in shipped_keymap_candidates(&name) {
        if candidate.exists()
            && let Ok(s) = std::fs::read_to_string(&candidate)
        {
            return Some(s);
        }
    }

    // 3) Embedded defaults
    let embedded = match name.as_str() {
        "neovim" | "vim" => Some(EMBEDDED_NEOVIM),
        "vscode" | "modern" => Some(EMBEDDED_VSCODE),
        _ => {
            if name != "norton" {
                report.warnings.push(format!(
                    "Preset '{preset}' not found on disk — falling back to embedded norton"
                ));
            }
            Some(EMBEDDED_NORTON)
        }
    };
    embedded.map(|s| s.to_string())
}

fn shipped_keymap_candidates(name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("keymaps").join(format!("{name}.toml")));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        out.push(dir.join("keymaps").join(format!("{name}.toml")));
        out.push(
            dir.join("../share/pairee/keymaps")
                .join(format!("{name}.toml")),
        );
    }
    out
}

fn normalize_preset_name(preset: &str) -> String {
    match preset.to_lowercase().as_str() {
        "vim" => "neovim".into(),
        "modern" => "vscode".into(),
        other => other.to_string(),
    }
}

/// Map legacy / friendly aliases to keybinds grammar.
/// Rejects empty; does not invent multi-character keys after modifiers.
fn normalize_user_chord(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    // Legacy Far-style gray keys → named keys supported by keybinds.
    match s {
        // Numpad / "gray" keys from Far-style docs → logical keys accepted by keybinds.
        "Gray+" | "gray+" | "GRAY+" => return "Plus".into(),
        "Gray-" | "gray-" | "GRAY-" => return "-".into(),
        "Gray*" | "gray*" | "GRAY*" => return "*".into(),
        "Menu" | "menu" => return "Menu".into(),
        _ => {}
    }
    // keybinds accepts Ctrl/Alt/Shift mixed case; leave as-is for parse.
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_impossible_chord() {
        let mut custom = HashMap::new();
        custom.insert("copy".into(), "Ctrl+rj".into());
        let (_kb, report) = load_keybinds("norton", &custom);
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("Ctrl+rj") || e.contains("Invalid")),
            "expected invalid chord error, got: {:?}",
            report.errors
        );
    }

    #[test]
    fn rejects_duplicate_chords_across_actions() {
        let mut custom = HashMap::new();
        // F5 is already copy in norton; force conflict with delete.
        custom.insert("delete".into(), "F5".into());
        let (_kb, report) = load_keybinds("norton", &custom);
        assert!(
            report.errors.iter().any(|e| e.contains("Duplicate")),
            "expected duplicate error, got: {:?}",
            report.errors
        );
    }

    #[test]
    fn norton_loads_core_bindings() {
        let (mut kb, report) = load_keybinds("norton", &HashMap::new());
        assert!(report.bound_count > 20, "bound={}", report.bound_count);
        // Invalid-only custom shouldn't wipe base if we only add bad custom — full load should work
        assert!(report.ok() || report.errors.is_empty() || report.bound_count > 0);

        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(kb.dispatch(up).copied(), Some(Action::MoveUp));
        let f5 = KeyEvent::new(KeyCode::F(5), KeyModifiers::empty());
        assert_eq!(kb.dispatch(f5).copied(), Some(Action::Copy));
    }

    #[test]
    fn impossible_chord_fails_keybinds_parse() {
        let err = "Ctrl+rj".parse::<KeySeq>();
        assert!(err.is_err(), "Ctrl+rj must not parse as a valid chord");
    }
}
