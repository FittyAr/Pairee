use super::report::KeymapLoadReport;
use crate::config::paths;
use crate::keybindings::embedded::{self, normalize_preset_name};
use std::path::PathBuf;

pub fn load_preset_toml(preset: &str, report: &mut KeymapLoadReport) -> Option<String> {
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
    let embedded = embedded::preset_toml(&name).unwrap_or_else(|| {
        report.warnings.push(format!(
            "Preset '{preset}' not found on disk — falling back to embedded norton"
        ));
        embedded::default_preset_toml()
    });
    Some(embedded.to_string())
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

/// Map legacy / friendly aliases to keybinds grammar.
pub fn normalize_user_chord(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    match s {
        "Gray+" | "gray+" | "GRAY+" => return "Plus".into(),
        "Gray-" | "gray-" | "GRAY-" => return "-".into(),
        "Gray*" | "gray*" | "GRAY*" => return "*".into(),
        "Menu" | "menu" => return "Menu".into(),
        _ => {}
    }
    let s = comma_alias(s).unwrap_or_else(|| s.to_string());
    shift_letter_to_uppercase(&s).unwrap_or(s)
}

/// `Comma` names the `,` key, which cannot be written literally because
/// commas separate alternative chords (`Ctrl+Comma` → `Ctrl+,`).
fn comma_alias(chord: &str) -> Option<String> {
    let (mods, key) = match chord.rsplit_once('+') {
        Some((mods, key)) => (Some(mods), key),
        None => (None, chord),
    };
    if !key.eq_ignore_ascii_case("comma") {
        return None;
    }
    Some(mods.map_or_else(|| ",".to_string(), |m| format!("{m}+,")))
}

/// `keybinds` only accepts `Shift` with named keys: a shifted letter is
/// written as the uppercase letter (`Ctrl+Shift+k` → `Ctrl+K`). Returns the
/// rewritten chord, or `None` when `chord` is not a shifted letter.
fn shift_letter_to_uppercase(chord: &str) -> Option<String> {
    let (mods, key) = chord.rsplit_once('+')?;
    let mut chars = key.chars();
    let letter = chars.next().filter(|c| c.is_alphabetic())?;
    if chars.next().is_some() {
        return None;
    }
    let parts: Vec<&str> = mods.split('+').collect();
    if !parts.iter().any(|m| m.eq_ignore_ascii_case("shift")) {
        return None;
    }
    let mut out: Vec<String> = parts
        .into_iter()
        .filter(|m| !m.eq_ignore_ascii_case("shift"))
        .map(str::to_string)
        .collect();
    out.push(letter.to_uppercase().collect());
    Some(out.join("+"))
}
