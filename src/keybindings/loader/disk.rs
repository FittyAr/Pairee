use super::report::KeymapLoadReport;
use crate::config::paths;
use std::path::PathBuf;

const EMBEDDED_NORTON: &str = include_str!("../../../keymaps/norton.toml");
const EMBEDDED_NEOVIM: &str = include_str!("../../../keymaps/neovim.toml");
const EMBEDDED_VSCODE: &str = include_str!("../../../keymaps/vscode.toml");

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

pub fn normalize_preset_name(preset: &str) -> String {
    match preset.to_lowercase().as_str() {
        "vim" => "neovim".into(),
        "modern" => "vscode".into(),
        other => other.to_string(),
    }
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
    s.to_string()
}
