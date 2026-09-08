//! Load and **validate** keymaps using the `keybinds` crate.

pub mod disk;
pub mod report;

pub use disk::normalize_user_chord;
pub use report::KeymapLoadReport;

use super::actions::Action;
use super::preset::parse_action_name;
use keybinds::{KeySeq, Keybind, Keybinds};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct PresetFile {
    bindings: HashMap<String, String>,
}

/// Build a validated [`Keybinds`] dispatcher for the active preset + user overrides.
pub fn load_keybinds(
    preset: &str,
    custom_bindings: &HashMap<String, String>,
) -> (Keybinds<Action>, KeymapLoadReport) {
    let mut report = KeymapLoadReport::default();
    let mut keybinds = Keybinds::default();
    let mut chord_owner: HashMap<String, String> = HashMap::new();

    let toml_src = disk::load_preset_toml(preset, &mut report);
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
                continue;
            }

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

    #[test]
    fn gray_plus_aliases_map_to_keybinds_names() {
        assert_eq!(normalize_user_chord("Gray+"), "Plus");
        assert_eq!(normalize_user_chord("gray-"), "-");
        assert_eq!(normalize_user_chord("GRAY*"), "*");
        assert!(
            "Plus".parse::<KeySeq>().is_ok(),
            "keybinds must accept Plus (Gray+ target)"
        );
        let mut custom = HashMap::new();
        custom.insert("select_group".into(), "Gray+".into());
        let (_kb, report) = load_keybinds("norton", &custom);
        assert!(
            !report.errors.iter().any(|e| e.contains("Gray+")),
            "Gray+ must be rewritten before parse: {:?}",
            report.errors
        );
    }

    #[test]
    fn custom_invalid_chord_appears_in_detail_lines() {
        let mut custom = HashMap::new();
        custom.insert("copy".into(), "Ctrl+rj".into());
        let (_kb, report) = load_keybinds("norton", &custom);
        let details = report.detail_lines().join("\n");
        assert!(details.contains("Ctrl+rj"), "details={details}");
        assert!(!report.ok());
    }
}
