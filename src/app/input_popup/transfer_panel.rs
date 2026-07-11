use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferTab, TransferViewMode};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let transfer = match &mut state.transfer {
        Some(t) => t,
        None => return Err(()),
    };

    if let Some((job_id, _, _)) = transfer.active_conflict_info {
        let resolution = match key.code {
            KeyCode::Char('o') | KeyCode::Char('O') => Some(crate::fs::transfer::conflict::ConflictResolution::Overwrite),
            KeyCode::Char('a') | KeyCode::Char('A') => Some(crate::fs::transfer::conflict::ConflictResolution::OverwriteOlder),
            KeyCode::Char('s') | KeyCode::Char('S') => Some(crate::fs::transfer::conflict::ConflictResolution::Skip),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(crate::fs::transfer::conflict::ConflictResolution::Rename),
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
    }

    match key.code {
        KeyCode::Esc => {
            // Minimizar a barra compacta
            transfer.view_mode = TransferViewMode::Minimized;
            state.active_popup = None;
            Ok(None)
        }
        KeyCode::Tab => {
            // Siguiente pestaña
            let next_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Options,
                TransferTab::Options => TransferTab::Status,
                TransferTab::Status => TransferTab::Log,
                TransferTab::Log => TransferTab::Queue,
                TransferTab::Queue => TransferTab::FileList,
            };
            transfer.active_tab = next_tab;
            Ok(None)
        }
        KeyCode::BackTab => {
            // Pestaña anterior
            let prev_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Queue,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
                TransferTab::Queue => TransferTab::Log,
            };
            transfer.active_tab = prev_tab;
            Ok(None)
        }
        KeyCode::Left => {
            let prev_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Queue,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
                TransferTab::Queue => TransferTab::Log,
            };
            transfer.active_tab = prev_tab;
            Ok(None)
        }
        KeyCode::Right => {
            let next_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Options,
                TransferTab::Options => TransferTab::Status,
                TransferTab::Status => TransferTab::Log,
                TransferTab::Log => TransferTab::Queue,
                TransferTab::Queue => TransferTab::FileList,
            };
            transfer.active_tab = next_tab;
            Ok(None)
        }
        KeyCode::Char('1') => {
            transfer.active_tab = TransferTab::FileList;
            Ok(None)
        }
        KeyCode::Char('2') => {
            transfer.active_tab = TransferTab::Options;
            Ok(None)
        }
        KeyCode::Char('3') => {
            transfer.active_tab = TransferTab::Status;
            Ok(None)
        }
        KeyCode::Char('4') => {
            transfer.active_tab = TransferTab::Log;
            Ok(None)
        }
        KeyCode::Char('5') => {
            transfer.active_tab = TransferTab::Queue;
            Ok(None)
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            // Alternar Pausa / Reanudación
            let is_paused = transfer.engine.queue.get_active()
                .map(|j| j.status == crate::fs::transfer::job::TransferJobStatus::Paused)
                .unwrap_or(false);

            if is_paused {
                transfer.engine.resume();
            } else {
                transfer.engine.pause();
            }
            Ok(None)
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            // Saltar archivo
            transfer.engine.skip_file();
            Ok(None)
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            // Cancelar
            transfer.engine.cancel();
            Ok(None)
        }
        KeyCode::Delete => {
            if transfer.active_tab == TransferTab::Queue {
                let jobs = transfer.engine.queue.get_all();
                if let Some(job) = jobs.get(transfer.queue_cursor) {
                    if transfer.engine.queue.remove(job.id) {
                        transfer.log_lines.push(format!("[{}] Job removed from queue", job.id));
                    }
                }
            }
            Ok(None)
        }
        KeyCode::Up => {
            if transfer.active_tab == TransferTab::FileList {
                transfer.file_list_cursor = transfer.file_list_cursor.saturating_sub(1);
            } else if transfer.active_tab == TransferTab::Queue {
                transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            }
            Ok(None)
        }
        KeyCode::Down => {
            if transfer.active_tab == TransferTab::FileList {
                transfer.file_list_cursor = transfer.file_list_cursor.saturating_add(1);
            } else if transfer.active_tab == TransferTab::Queue {
                let max_idx = transfer.engine.queue.get_all().len().saturating_sub(1);
                if transfer.queue_cursor < max_idx {
                    transfer.queue_cursor += 1;
                }
            }
            Ok(None)
        }
        _ => Err(()),
    }
}
