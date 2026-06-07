use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// Active preset profile ("norton", "vim", "modern")
    pub preset: String,
    /// Custom key overrides mapping Action names (e.g. "copy") to key strings (e.g. "Ctrl+C")
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
