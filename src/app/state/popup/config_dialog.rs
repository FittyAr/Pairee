#[derive(Debug, Clone)]
pub struct ConfigurationDialogState {
    pub active_tab: usize,
    pub cursor_idx: usize,
    pub editing_value: bool,
    pub edit_buffer: String,
    pub settings: Box<crate::config::settings::Settings>,
    pub focus_on_tabs: bool,
}
