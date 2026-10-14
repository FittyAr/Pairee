//! Navigation and options editing for transfer panel.

use crate::app::state::transfer_options::OptionRow;
use crate::app::state::{TransferTab, TransferUIState};
use crate::fs::transfer::job::TransferJobStatus;
use crossterm::event::KeyCode;

pub fn handle_navigation(transfer: &mut TransferUIState, code: KeyCode) -> bool {
    match code {
        KeyCode::Tab | KeyCode::Right => {
            transfer.active_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Options,
                TransferTab::Options => TransferTab::Status,
                TransferTab::Status => TransferTab::Log,
                TransferTab::Log => TransferTab::FileList,
            };
            true
        }
        KeyCode::BackTab | KeyCode::Left => {
            transfer.active_tab = match transfer.active_tab {
                TransferTab::FileList => TransferTab::Log,
                TransferTab::Options => TransferTab::FileList,
                TransferTab::Status => TransferTab::Options,
                TransferTab::Log => TransferTab::Status,
            };
            true
        }
        KeyCode::Char('1') => {
            transfer.active_tab = TransferTab::FileList;
            true
        }
        KeyCode::Char('2') => {
            transfer.active_tab = TransferTab::Options;
            true
        }
        KeyCode::Char('3') => {
            transfer.active_tab = TransferTab::Status;
            true
        }
        KeyCode::Char('4') => {
            transfer.active_tab = TransferTab::Log;
            true
        }
        KeyCode::Up => {
            transfer.queue_cursor = transfer.queue_cursor.saturating_sub(1);
            true
        }
        KeyCode::Down => {
            let max_idx = transfer.engine.queue.len().saturating_sub(1);
            if transfer.queue_cursor < max_idx {
                transfer.queue_cursor += 1;
            }
            true
        }
        KeyCode::PageUp => {
            if transfer.active_tab == TransferTab::FileList {
                transfer.file_list_cursor = transfer.file_list_cursor.saturating_sub(1);
            } else if transfer.active_tab == TransferTab::Options {
                transfer.options_cursor = transfer.options_cursor.saturating_sub(1);
            }
            true
        }
        KeyCode::PageDown => {
            if transfer.active_tab == TransferTab::FileList {
                let jobs = transfer.engine.queue.get_all();
                let total_files = if let Some(job) = jobs.get(transfer.queue_cursor) {
                    job.results.failed_files.len()
                        + job.results.skipped_files.len()
                        + job.results.completed_files.len()
                } else {
                    0
                };
                let max_idx = total_files.saturating_sub(1);
                if transfer.file_list_cursor < max_idx {
                    transfer.file_list_cursor += 1;
                }
            } else if transfer.active_tab == TransferTab::Options
                && transfer.options_cursor + 1 < OptionRow::ALL.len()
            {
                transfer.options_cursor += 1;
            }
            true
        }
        _ => false,
    }
}

/// Enter on an Options row: steps its value. A job that already started
/// only takes the live rows ([`OptionRow::is_live`]): it shares them with
/// its worker through `TransferJob::live`; the others are fixed then.
pub fn handle_options_toggle(transfer: &mut TransferUIState) {
    let Some(row) = OptionRow::at(transfer.options_cursor) else {
        return;
    };
    if row == OptionRow::PostAction {
        transfer.post_action = next_post_action(&transfer.post_action);
        return;
    }
    let jobs = transfer.engine.queue.get_all();
    let Some(job) = jobs.get(transfer.queue_cursor) else {
        return;
    };
    if !row.is_live() && job.status != TransferJobStatus::Queued {
        return;
    }
    transfer.engine.queue.update_job(job.id, |j| {
        row.edit(&mut j.options);
        j.live.publish(&j.options);
    });
}

/// The post action after `current` in the Options row. "Eject drive" and
/// "Run script" are not offered: nothing lets the user choose the drive or
/// the script, and an empty one fell back to an arbitrary disk.
fn next_post_action(
    current: &crate::fs::transfer::post_action::PostAction,
) -> crate::fs::transfer::post_action::PostAction {
    use crate::fs::transfer::post_action::PostAction as P;
    match current {
        P::None => P::Shutdown,
        P::Shutdown => P::Sleep,
        P::Sleep => P::Hibernate,
        P::Hibernate => P::CloseApp,
        P::EjectDrive(_) | P::RunScript(_) | P::CloseApp => P::None,
    }
}

#[cfg(test)]
mod tests {
    use super::next_post_action;
    use crate::fs::transfer::post_action::PostAction as P;

    #[test]
    fn post_actions_cycle_without_unset_targets() {
        let mut seen = vec![P::None];
        loop {
            let next = next_post_action(seen.last().unwrap());
            if next == P::None {
                break;
            }
            seen.push(next);
        }
        assert_eq!(
            seen,
            [P::None, P::Shutdown, P::Sleep, P::Hibernate, P::CloseApp]
        );
    }
}
