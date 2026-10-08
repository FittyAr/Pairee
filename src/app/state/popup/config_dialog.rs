#[derive(Debug, Clone)]
pub struct ConfigurationDialogState {
    pub active_tab: usize,
    pub cursor_idx: usize,
    pub editing_value: bool,
    pub edit_buffer: String,
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
            editing_value: false,
            edit_buffer: String::new(),
            settings: Box::new(settings.clone()),
            focus_on_tabs,
        }
    }
}
