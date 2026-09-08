//! Conflict resolution keyboard handler for active transfer jobs.

use crate::app::state::TransferUIState;
use crossterm::event::{KeyCode, KeyEvent};
use uuid::Uuid;

pub fn handle_conflict(
    transfer: &mut TransferUIState,
    key: KeyEvent,
    job_id: Uuid,
) -> Result<Option<crate::keybindings::Action>, ()> {
    let resolution = match key.code {
        KeyCode::Char('o') => Some(crate::fs::transfer::conflict::ConflictResolution::Overwrite),
        KeyCode::Char('O') => Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteAll),
        KeyCode::Char('a') => {
            Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder)
        }
        KeyCode::Char('A') => {
            Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteOlderAll)
        }
        KeyCode::Char('s') => Some(crate::fs::transfer::conflict::ConflictResolution::Skip),
        KeyCode::Char('S') => Some(crate::fs::transfer::conflict::ConflictResolution::SkipAll),
        KeyCode::Char('r') => Some(crate::fs::transfer::conflict::ConflictResolution::Rename),
        KeyCode::Char('R') => Some(crate::fs::transfer::conflict::ConflictResolution::RenameAll),
        KeyCode::Char('x') | KeyCode::Char('X') => {
            Some(crate::fs::transfer::conflict::ConflictResolution::Cancel)
        }
        _ => None,
    };

    if let Some(res) = resolution {
        let jobs = transfer.engine.queue.get_all();
        if let Some(job) = jobs.iter().find(|j| j.id == job_id) {
            let mut guard = job.active_conflict.lock().unwrap();
            *guard = Some(res);
        }
        transfer.active_conflict_info = None;
        return Ok(None);
    }
    if key.code != KeyCode::Esc {
        return Ok(None);
    }
    Ok(None)
}
