use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for keybinding preset selection and per-user overrides.
///
/// ## Preset files
/// Each preset is a TOML file in the `keymaps/` subdirectory of the Pairee
/// config folder (e.g. `%APPDATA%\pairee\config\keymaps\` on Windows).
///
/// Built-in presets shipped with Pairee:
/// - `"norton"` — Norton Commander / Far Manager classic layout (default)
/// - `"neovim"` — Neovim-style navigation (h/j/k/l, gg/G, :, /, Ctrl+d/u)
/// - `"vscode"` — VS Code Explorer-style shortcuts (Ctrl+C copy, Delete delete, …)
///
/// ## Custom presets
/// Create any file `<name>.toml` in the `keymaps/` directory and set
/// `preset = "<name>"` here to activate it. The file must have a `[bindings]`
/// table mapping snake_case action names to key strings, for example:
///
/// ```toml
/// [bindings]
/// move_up   = "k"
/// move_down = "j"
/// copy      = "F5"
/// ```
///
/// ## Per-binding overrides
/// `custom_bindings` overlays individual key assignments on top of the active
/// preset without replacing the whole file. This is the recommended way to
/// tweak one or two shortcuts without duplicating an entire preset file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// Active preset profile name. Must match a file in the `keymaps/` directory.
    /// Built-in values: `"norton"`, `"neovim"`, `"vscode"`.
    pub preset: String,
    /// Per-binding overrides applied on top of the active preset.
    /// Maps action names (e.g. `"copy"`) to key strings (e.g. `"Ctrl+c"`).
    pub custom_bindings: HashMap<String, String>,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            preset: "norton".to_string(),
            custom_bindings: HashMap::new(),
        }
    }
}
