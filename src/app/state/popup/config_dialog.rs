use crate::app::text_input::TextField;
use crate::config::AppConfig;
use crate::config::settings::Settings;
use std::ops::{Deref, DerefMut};

/// What the configuration dialog edits: a copy of the settings plus the
/// keymap preset, whose single source of truth is `keybindings.preset`.
#[derive(Debug, Clone)]
pub struct ConfigDraft {
    pub settings: Settings,
    pub keymap_preset: String,
}

impl ConfigDraft {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            settings: config.settings.clone(),
            keymap_preset: config.keybindings.preset.clone(),
        }
    }
}

impl Deref for ConfigDraft {
    type Target = Settings;
    fn deref(&self) -> &Settings {
        &self.settings
    }
}

impl DerefMut for ConfigDraft {
    fn deref_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
}

#[derive(Debug, Clone)]
pub struct ConfigurationDialogState {
    pub active_tab: usize,
    pub cursor_idx: usize,
    /// Text of the setting row being edited, if any.
    pub edit: Option<TextField>,
    pub draft: Box<ConfigDraft>,
    pub focus_on_tabs: bool,
}

impl ConfigurationDialogState {
    /// The dialog on `active_tab`, row `cursor_idx`, editing a copy of `config`.
    pub fn new(
        config: &AppConfig,
        active_tab: usize,
        cursor_idx: usize,
        focus_on_tabs: bool,
    ) -> Self {
        Self {
            active_tab,
            cursor_idx,
            edit: None,
            draft: Box::new(ConfigDraft::new(config)),
            focus_on_tabs,
        }
    }
}
