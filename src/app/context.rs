use crate::config::AppConfig;
use crate::keybindings::KeybindingResolver;
use std::sync::Arc;

pub struct AppContext {
    /// Active settings, key mappings, and color schemes
    pub config: AppConfig,
    /// Loaded keyboard translation resolver. `Arc`-shared
    /// between the main loop (reads on every keypress) and the
    /// plugin runtime (writes on every manifest entry and every
    /// Lua `pairee.keybindings.bind` call); the Lua closures
    /// need an owned handle so they can outlive the
    /// `bind_runtime` call that created them.
    pub resolver: Arc<KeybindingResolver>,
}

impl AppContext {
    pub fn new(config: AppConfig) -> Self {
        let resolver = Arc::new(KeybindingResolver::new(&config));
        Self { config, resolver }
    }
}
