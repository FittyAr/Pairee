use crate::app::state::AppState;
use crate::app::state::popup::CreateKind;
use crate::app::state::popup::MkDirPrompt;

/// Opens the create prompt for a folder, a file or either.
pub fn handle(state: &mut AppState, kind: CreateKind) -> bool {
    state.dialogs.replace(MkDirPrompt {
        input: Default::default(),
        cursor_idx: 0,
        process_multiple: false,
        kind,
    });
    true
}
