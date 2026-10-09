//! The preset keymaps shipped inside the binary: the only `include_str!` of
//! `keymaps/*.toml`, used both to seed the user's `keymaps/` folder and as
//! the last fallback of the loader.

use std::path::{Path, PathBuf};

const NORTON: &str = include_str!("../../keymaps/norton.toml");
const STANDARD: &str = include_str!("../../keymaps/standard.toml");
const NEOVIM: &str = include_str!("../../keymaps/neovim.toml");
const YAZI: &str = include_str!("../../keymaps/yazi.toml");
const BASE: &str = include_str!("../../keymaps/base.toml");
const VI: &str = include_str!("../../keymaps/vi.toml");

/// Built-in presets with their shipped TOML, in display order.
pub const PRESETS: &[(&str, &str)] = &[
    ("norton", NORTON),
    ("standard", STANDARD),
    ("neovim", NEOVIM),
    ("yazi", YAZI),
];

/// Shipped presets that only exist to be extended (not selectable, not seeded).
const HIDDEN: &[(&str, &str)] = &[("base", BASE), ("vi", VI)];

/// Canonical name of a preset: lower case, with the legacy names (`vim`,
/// `vscode`, `modern`) mapped to the built-in they stand for.
pub fn normalize_preset_name(preset: &str) -> String {
    match preset.to_lowercase().as_str() {
        "vim" | "nvim" => "neovim".into(),
        "vscode" | "modern" => "standard".into(),
        other => other.to_string(),
    }
}

/// Shipped TOML for a built-in preset name (`None` for custom presets).
pub fn preset_toml(preset: &str) -> Option<&'static str> {
    let name = normalize_preset_name(preset);
    PRESETS
        .iter()
        .chain(HIDDEN)
        .find(|(builtin, _)| *builtin == name)
        .map(|(_, toml)| *toml)
}

/// Selectable presets: the built-in ones, then the user's own files in
/// `keymaps_dir` (by name), without the hidden layers.
pub fn available_presets(keymaps_dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = PRESETS.iter().map(|(n, _)| n.to_string()).collect();
    let mut custom: Vec<String> = std::fs::read_dir(keymaps_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|f| f.strip_suffix(".toml").map(str::to_string))
        .filter(|n| preset_toml(n).is_none() && normalize_preset_name(n) == *n)
        .collect();
    custom.sort();
    names.extend(custom);
    names
}

/// Where the seeder offers a newer shipped `name` preset when the user's
/// `keymaps/<name>.toml` differs from it.
pub fn pending_update_path(keymaps_dir: &Path, name: &str) -> PathBuf {
    keymaps_dir.join(format!("{name}.toml.new"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_presets_add_user_files_after_the_builtins() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "mine.toml",
            "norton.toml",
            "vscode.toml",
            "base.toml",
            "notes.txt",
        ] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        assert_eq!(
            available_presets(dir.path()),
            ["norton", "standard", "neovim", "yazi", "mine"]
        );
    }

    #[test]
    fn aliases_resolve_to_builtins() {
        assert_eq!(preset_toml("vim"), Some(NEOVIM));
        assert_eq!(preset_toml("Modern"), Some(STANDARD));
        assert_eq!(preset_toml("vscode"), Some(STANDARD));
        assert_eq!(preset_toml("vi"), Some(VI));
        assert_eq!(preset_toml("norton"), Some(NORTON));
        assert_eq!(preset_toml("my-custom"), None);
    }
}
