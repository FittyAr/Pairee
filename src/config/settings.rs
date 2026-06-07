use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Whether to display hidden files/directories (starting with `.`)
    pub show_hidden: bool,
    /// The external editor command to trigger for F4 Edit (e.g. "nano", "vim", "notepad")
    pub default_editor: String,
    /// Toggle terminal mouse interactions
    pub mouse_support: bool,
    /// Active keybinding preset profile: "norton", "vim", "modern", "custom"
    pub keybinding_preset: String,
    /// The name of the active theme (corresponds to theme TOML files or defaults)
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_editor: if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                "nano".to_string()
            },
            mouse_support: true,
            keybinding_preset: "norton".to_string(),
            theme: "slate".to_string(),
        }
    }
}
