use crate::config::AppConfig;
use crate::keybindings::KeybindingResolver;

pub struct AppContext {
    /// Active settings, key mappings, and color schemes
    pub config: AppConfig,
    /// Loaded keyboard translation resolver
    pub resolver: KeybindingResolver,
}

impl AppContext {
    pub fn new(config: AppConfig) -> Self {
        let resolver = KeybindingResolver::new(&config);
        Self { config, resolver }
    }
}
