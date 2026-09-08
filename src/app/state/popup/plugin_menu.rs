#[derive(Debug, Clone)]
pub struct PluginMenuState {
    pub active_tab: usize,
    pub cursor_idx: usize,
    pub installed: Vec<(String, String, bool, bool, Option<String>)>,
    pub all_registry: Vec<(String, String, String, String)>,
    pub registry: Vec<(String, String, String, String)>,
    pub search_query: String,
    pub is_searching: bool,
    pub editing_query: bool,
    pub dev_results: String,
    pub dev_wizard_step: usize,
    pub dev_wizard_data: Vec<String>,
    pub installed_loading: bool,
    pub installed_loading_status: String,
    pub dev_loading: bool,
    pub dev_loading_status: String,
    pub dev_loading_progress: Option<(usize, usize)>,
}
