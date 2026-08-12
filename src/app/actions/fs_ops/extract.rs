use crate::app::state::AppState;
use crate::fs::transfer::job::TransferOperation;
use crate::fs::transfer::options::TransferOptions;
use crate::fs::transfer::submit_simple;

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let dest = state.get_passive_panel().current_path.clone();
        let archive = entry.path.clone();
        submit_simple(
            state,
            TransferOperation::Extract,
            vec![archive],
            dest,
            TransferOptions::default(),
            None,
            None,
        );
    }
    true
}
