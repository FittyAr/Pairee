use crate::config::AppConfig;
use crate::keybindings::KeybindingResolver;
use crate::ui::highlight::Highlighter;
use crate::ui::render_cache::RenderCache;
use crate::ui::resolved_theme::ResolvedTheme;
use std::sync::Arc;

pub struct AppContext {
    /// Active settings, key mappings, and color schemes
    pub config: AppConfig,
    /// Loaded keyboard translation resolver
    pub resolver: KeybindingResolver,
    /// Render values derived from `config` (rebuilt when it changes).
    render: RenderCache,
}

impl AppContext {
    pub fn new(config: AppConfig) -> Self {
        let resolver = KeybindingResolver::new(&config);
        Self {
            config,
            resolver,
            render: RenderCache::default(),
        }
    }

    /// The active theme with its colors parsed (once per theme change).
    pub fn theme(&self) -> Arc<ResolvedTheme> {
        self.render.theme(&self.config.theme)
    }

    /// The user's "Files highlighting" rules, compiled once per change.
    pub fn highlighter(&self) -> Arc<Highlighter> {
        self.render
            .highlighter(&self.config.settings.highlight_rules)
    }
}
