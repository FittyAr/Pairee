use crate::app::state::{AppState, LinkKind, PopupType};
use std::path::Path;

pub fn handle(state: &mut AppState) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active.entries.get(active.cursor_index)
        && entry.name != ".."
    {
        state.dialogs.replace(PopupType::CreateLinkPrompt {
            src: entry.path.clone(),
            dest_input: entry.name.as_str().into(),
            kind: LinkKind::Symbolic,
        });
    }
    true
}

/// Creates a `kind` link at `dest` to `src` and journals it for undo.
pub fn make(state: &mut AppState, src: &Path, dest: &Path, kind: LinkKind) -> anyhow::Result<()> {
    crate::fs::create_link(src, dest, kind)?;
    state.journal.record(crate::fs::journal::FsCommand::Link {
        link: dest.to_path_buf(),
        target: src.to_path_buf(),
        kind,
    });
    Ok(())
}
