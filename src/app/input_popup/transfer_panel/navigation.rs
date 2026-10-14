//! Navigation and options editing for transfer panel.

use crate::app::state::{TransferTab, TransferUIState};
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
            let max_idx = transfer.engine.queue.get_all().len().saturating_sub(1);
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
            } else if transfer.active_tab == TransferTab::Options && transfer.options_cursor < 11 {
                transfer.options_cursor += 1;
            }
            true
        }
        _ => false,
    }
}

pub fn handle_options_toggle(transfer: &mut TransferUIState) {
    let jobs = transfer.engine.queue.get_all();
    if let Some(job) = jobs.get(transfer.queue_cursor) {
        let job_id = job.id;
        transfer
            .engine
            .queue
            .update_job(job_id, |j| match transfer.options_cursor {
                0 => j.options.direct_io = !j.options.direct_io,
                1 => j.options.verify_after_copy = !j.options.verify_after_copy,
                2 => j.options.preserve_timestamps = !j.options.preserve_timestamps,
                3 => j.options.preserve_attributes = !j.options.preserve_attributes,
                4 => {}
                5 => {
                    j.options.buffer_size = match j.options.buffer_size {
                        crate::fs::transfer::options::BufferSize::_64KB => {
                            crate::fs::transfer::options::BufferSize::_256KB
                        }
                        crate::fs::transfer::options::BufferSize::_256KB => {
                            crate::fs::transfer::options::BufferSize::_1MB
                        }
                        crate::fs::transfer::options::BufferSize::_1MB => {
                            crate::fs::transfer::options::BufferSize::_4MB
                        }
                        crate::fs::transfer::options::BufferSize::_4MB => {
                            crate::fs::transfer::options::BufferSize::_64KB
                        }
                    };
                }
                6 => {
                    j.options.hash_algorithm = j.options.hash_algorithm.next();
                }
                7 => j.options.preserve_acl = !j.options.preserve_acl,
                8 => j.options.preserve_streams = !j.options.preserve_streams,
                9 => j.options.skip_symlinks = !j.options.skip_symlinks,
                10 => j.options.follow_symlinks = !j.options.follow_symlinks,
                11 => {
                    j.options.limit_bandwidth_rate = match j.options.limit_bandwidth_rate {
                        None => Some(1_048_576),
                        Some(1_048_576) => Some(10_485_760),
                        Some(10_485_760) => Some(52_428_800),
                        Some(_) => None,
                    };
                }
                _ => {}
            });
        if transfer.options_cursor == 4 {
            transfer.post_action = next_post_action(&transfer.post_action);
        }
    }
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
