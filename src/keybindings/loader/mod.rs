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
    let toml_src = disk::load_preset_toml(preset, &mut report);
    build_keybinds(preset, toml_src.as_deref(), custom_bindings, report)
}

/// Validates the preset file `toml_src` plus `custom_bindings` into a
/// dispatcher, recording every problem in `report`.
pub fn build_keybinds(
    preset: &str,
    toml_src: Option<&str>,
    custom_bindings: &HashMap<String, String>,
    mut report: KeymapLoadReport,
) -> (Keybinds<Action>, KeymapLoadReport) {
    let pairs = collect_pairs(preset, toml_src, custom_bindings, &mut report);
    let mut binder = Binder::default();
    for (action_name, keys_field, strict) in pairs {
        let Some(action) = parse_action_name(&action_name) else {
            if strict {
                report
                    .warnings
                    .push(format!("Unknown action '{action_name}' — skipped"));
            }
            continue;
        };
        for raw_key in keys_field.split(',') {
            let chord = normalize_user_chord(raw_key.trim());
            if !chord.is_empty() {
                binder.bind(&chord, action, &action_name, strict, &mut report);
            }
        }
    }
    if report.bound_count == 0 {
        report
            .errors
            .push("No key bindings loaded — keymap is empty after validation".into());
    }
    (binder.keybinds, report)
}

/// `(action, keys, strict)` rows: the preset file, then the user's custom
/// bindings, then (non-strict) shipped defaults for a built-in preset whose
/// on-disk copy predates the action. Non-strict rows never override or
/// conflict with the user's file and fail silently.
fn collect_pairs(
    preset: &str,
    toml_src: Option<&str>,
    custom_bindings: &HashMap<String, String>,
    report: &mut KeymapLoadReport,
) -> Vec<(String, String, bool)> {
    let mut pairs: Vec<(String, String, bool)> = Vec::new();
    let mut fallback: Vec<(String, String, bool)> = Vec::new();
    if let Some(content) = toml_src {
        match toml::from_str::<PresetFile>(content) {
            Ok(file) => {
                if let Some(embedded) = crate::keybindings::embedded::preset_toml(preset)
                    && let Ok(defaults) = toml::from_str::<PresetFile>(embedded)
                {
                    fallback = defaults
                        .bindings
                        .into_iter()
                        .filter(|(action, _)| !file.bindings.contains_key(action))
                        .map(|(action, keys)| (action, keys, false))
                        .collect();
                }
                pairs.extend(file.bindings.into_iter().map(|(a, k)| (a, k, true)));
            }
            Err(e) => report
                .errors
                .push(format!("Failed to parse keymap for preset '{preset}': {e}")),
        }
    }
    for (action, keys) in custom_bindings {
        pairs.push((action.clone(), keys.clone(), true));
    }
    fallback.sort();
    pairs.extend(fallback);
    pairs
}

/// Accumulates bindings, rejecting invalid and conflicting chords.
#[derive(Default)]
struct Binder {
    keybinds: Keybinds<Action>,
    chord_owner: HashMap<String, String>,
}

impl Binder {
    fn bind(
        &mut self,
        chord: &str,
        action: Action,
        action_name: &str,
        strict: bool,
        report: &mut KeymapLoadReport,
    ) {
        let seq: KeySeq = match chord.parse() {
            Ok(s) => s,
            Err(e) => {
                if strict {
                    report.errors.push(format!(
                        "Invalid key chord '{chord}' for action '{action_name}': {e}"
                    ));
                }
                return;
            }
        };
        let chord_key = seq.to_string();
        if let Some(prev) = self.chord_owner.get(&chord_key) {
            if strict && prev != action_name {
                report.errors.push(format!(
                    "Duplicate key chord '{chord_key}': already bound to '{prev}', cannot also bind '{action_name}'"
                ));
            }
            return;
        }
        if self
            .keybinds
            .as_slice()
            .iter()
            .any(|b| b.seq == seq && b.action != action)
        {
            if strict {
                report.errors.push(format!(
                    "Duplicate key chord '{chord_key}' conflicts with an existing binding"
                ));
            }
            return;
        }
        self.keybinds.push(Keybind::new(seq, action));
        self.chord_owner.insert(chord_key, action_name.to_string());
        report.bound_count += 1;
    }
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
    fn hotlist_and_git_panel_are_bound_in_every_builtin_preset() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        for preset in ["norton", "neovim", "vscode"] {
            let (mut kb, _) = load_keybinds(preset, &HashMap::new());
            let hotlist = KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL);
            assert_eq!(
                kb.dispatch(hotlist).copied(),
                Some(Action::Hotlist),
                "{preset}"
            );
            let git = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::ALT);
            assert_eq!(
                kb.dispatch(git).copied(),
                Some(Action::OpenGitPanel),
                "{preset}"
            );
        }
    }

    /// Every chord of a shipped preset, as `(normalized chord, action)`;
    /// panics on chords the `keybinds` crate rejects.
    fn shipped_chords(src: &str) -> Vec<(String, String)> {
        let file: PresetFile = toml::from_str(src).unwrap();
        let mut chords = Vec::new();
        for (action, keys) in file.bindings {
            for raw in keys.split(',').map(str::trim).filter(|k| !k.is_empty()) {
                let seq: KeySeq = normalize_user_chord(raw).parse().unwrap();
                chords.push((seq.to_string(), action.clone()));
            }
        }
        chords.sort();
        chords
    }

    #[test]
    fn shipped_presets_load_without_errors_or_duplicate_chords() {
        for preset in ["norton", "neovim", "vscode"] {
            let src = crate::keybindings::embedded::preset_toml(preset).unwrap();
            let (_, report) =
                build_keybinds(preset, Some(src), &HashMap::new(), Default::default());
            assert!(
                report.errors.is_empty() && report.warnings.is_empty(),
                "{preset}: {:?} {:?}",
                report.errors,
                report.warnings
            );
            let chords = shipped_chords(src);
            let dups: Vec<_> = chords
                .windows(2)
                .filter(|w| w[0].0 == w[1].0)
                .map(|w| format!("{} ({} / {})", w[0].0, w[0].1, w[1].1))
                .collect();
            assert!(dups.is_empty(), "{preset}: duplicate chords {dups:?}");
        }
    }

    #[test]
    fn shifted_letters_are_written_uppercase() {
        assert_eq!(normalize_user_chord("Ctrl+Shift+k"), "Ctrl+K");
        assert_eq!(normalize_user_chord("shift+Alt+p"), "Alt+P");
        assert_eq!(normalize_user_chord("Shift+F6"), "Shift+F6");
        assert_eq!(normalize_user_chord("Alt+Shift+PageUp"), "Alt+Shift+PageUp");
        assert_eq!(normalize_user_chord("Ctrl+Comma"), "Ctrl+,");
        assert!("Ctrl+,".parse::<KeySeq>().is_ok());
        let mut custom = HashMap::new();
        custom.insert("which_key".into(), "Ctrl+Shift+k".into());
        let (mut kb, report) = load_keybinds("norton", &custom);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key = KeyEvent::new(
            KeyCode::Char('K'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(kb.dispatch(key).copied(), Some(Action::WhichKey));
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
