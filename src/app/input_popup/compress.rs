use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::CompressPrompt { input, .. }) = state.dialogs.top_mut() else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(PopupType::CompressPrompt {
                input,
                targets,
                dest_dir,
            }) = state.dialogs.pop()
            {
                submit(state, input.into_text(), targets, dest_dir);
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

/// Queues `<dest_dir>/<name>.zip` (nothing for an empty name).
fn submit(
    state: &mut AppState,
    mut name: String,
    targets: Vec<std::path::PathBuf>,
    dest_dir: std::path::PathBuf,
) {
    state.dialogs.clear();
    if name.is_empty() {
        return;
    }
    if !name.ends_with(".zip") {
        name.push_str(".zip");
    }
    crate::fs::transfer::submit_simple(
        state,
        crate::fs::transfer::job::TransferOperation::Compress,
        targets,
        dest_dir.join(name),
        crate::fs::transfer::options::TransferOptions::default(),
        None,
        None,
    );
}
