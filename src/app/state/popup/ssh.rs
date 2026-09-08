use crate::app::state::types::ActivePanel;

#[derive(Debug, Clone)]
pub struct SshConnectPromptState {
    pub panel: ActivePanel,
    pub input_name: String,
    pub input_host: String,
    pub input_port: String,
    pub input_user: String,
    pub input_pass: String,
    pub input_key_path: String,
    pub cursor_idx: usize,
    pub selected_preset_idx: Option<usize>,
}
