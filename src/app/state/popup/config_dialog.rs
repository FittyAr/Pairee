use crate::app::text_input::TextField;

#[derive(Debug, Clone)]
pub struct ConfigurationDialogState {
    pub active_tab: usize,
    pub cursor_idx: usize,
    /// Text of the setting row being edited, if any.
    pub edit: Option<TextField>,
    pub settings: Box<crate::config::settings::Settings>,
    pub focus_on_tabs: bool,
}

impl ConfigurationDialogState {
    /// The dialog on `active_tab`, row `cursor_idx`, editing a copy of `settings`.
    pub fn new(
        settings: &crate::config::settings::Settings,
        active_tab: usize,
        cursor_idx: usize,
        focus_on_tabs: bool,
    ) -> Self {
        Self {
            active_tab,
            cursor_idx,
            edit: None,
            settings: Box::new(settings.clone()),
            focus_on_tabs,
        }
    }
}
