//! The preset keymaps shipped inside the binary: the only `include_str!` of
//! `keymaps/*.toml`, used both to seed the user's `keymaps/` folder and as
//! the last fallback of the loader.

const NORTON: &str = include_str!("../../keymaps/norton.toml");
const NEOVIM: &str = include_str!("../../keymaps/neovim.toml");
const VSCODE: &str = include_str!("../../keymaps/vscode.toml");

/// Built-in presets with their shipped TOML, in display order.
pub const PRESETS: &[(&str, &str)] = &[("norton", NORTON), ("neovim", NEOVIM), ("vscode", VSCODE)];

/// Canonical name of a preset: lower case, with the legacy aliases
/// (`vim`, `modern`) mapped to the built-in they stand for.
pub fn normalize_preset_name(preset: &str) -> String {
    match preset.to_lowercase().as_str() {
        "vim" => "neovim".into(),
        "modern" => "vscode".into(),
        other => other.to_string(),
    }
}

/// Shipped TOML for a built-in preset name (`None` for custom presets).
pub fn preset_toml(preset: &str) -> Option<&'static str> {
    let name = normalize_preset_name(preset);
    PRESETS
        .iter()
        .find(|(builtin, _)| *builtin == name)
        .map(|(_, toml)| *toml)
}

/// Shipped TOML of the default preset (norton).
pub fn default_preset_toml() -> &'static str {
    NORTON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_builtins() {
        assert_eq!(preset_toml("vim"), Some(NEOVIM));
        assert_eq!(preset_toml("Modern"), Some(VSCODE));
        assert_eq!(preset_toml("norton"), Some(NORTON));
        assert_eq!(preset_toml("my-custom"), None);
    }
}
