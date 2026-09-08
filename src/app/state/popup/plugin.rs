#[derive(Debug, Clone)]
pub enum PluginDialog {
    Confirm {
        title: String,
        msg: String,
        /// 0 = Yes, 1 = No
        cursor_idx: usize,
        position: Option<crate::plugin::manager::DialogPosition>,
    },
    Input {
        title: String,
        input: String,
        obscure: bool,
        position: Option<crate::plugin::manager::DialogPosition>,
    },
    Which {
        candidates: Vec<crate::plugin::manager::WhichCandidate>,
        silent: bool,
        position: Option<crate::plugin::manager::DialogPosition>,
    },
}
