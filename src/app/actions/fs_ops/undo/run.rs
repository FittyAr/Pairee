//! Runs a checked undo/redo command through the paths the original
//! operations use: the multi-rename executor, Transfer Engine jobs, and the
//! make-folder / link primitives. What actually ran is filed in the journal
//! when it finishes (see [`super::record`]).

use crate::app::actions::fs_ops::multi_rename::run_steps;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::journal::{Direction, FileBatch, FsCommand};
use crate::fs::transfer::job::{TransferJob, TransferOperation};
use crate::fs::transfer::options::TransferOptions;
use std::path::PathBuf;

/// Starts `command` (already checked) on behalf of an undo/redo.
pub(super) fn run(
    state: &mut AppState,
    context: &AppContext,
    direction: Direction,
    command: FsCommand,
) {
    let result = match &command {
        FsCommand::Rename { steps, ssh } => {
            state.journal.begin_rename(direction);
            run_steps(state, steps.clone(), ssh.clone());
            return;
        }
        FsCommand::Move(batch) => {
            let job = pairs_job(TransferOperation::Move, batch, context);
            return submit(state, direction, job, command);
        }
        FsCommand::Copy(batch) => {
            let job = pairs_job(TransferOperation::Copy, batch, context);
            return submit(state, direction, job, command);
        }
        FsCommand::RemoveCopies(batch) => {
            let copies = batch.files.iter().map(|f| f.to.clone()).collect();
            let job = TransferJob::new(
                TransferOperation::Delete,
                copies,
                PathBuf::new(),
                TransferOptions::default(),
            )
            .with_prune_dirs(batch.prune_dirs.clone());
            return submit(state, direction, job, command);
        }
        FsCommand::Trash { paths } => {
            let options = TransferOptions {
                delete_to_recycle_bin: true,
                ..TransferOptions::default()
            };
            let job = TransferJob::new(
                TransferOperation::Delete,
                paths.clone(),
                PathBuf::new(),
                options,
            );
            return submit(state, direction, job, command);
        }
        FsCommand::Restore { paths } => {
            let job = TransferJob::new(
                TransferOperation::Restore,
                paths.clone(),
                PathBuf::new(),
                TransferOptions::default(),
            );
            return submit(state, direction, job, command);
        }
        FsCommand::MakeDir { path } => {
            crate::fs::create_directory(path, false).map_err(|e| e.to_string())
        }
        FsCommand::RemoveDir { path } => std::fs::remove_dir(path).map_err(|e| e.to_string()),
        FsCommand::Link { link, target, kind } => {
            crate::fs::create_link(target, link, *kind).map_err(|e| e.to_string())
        }
        FsCommand::Unlink { link, .. } => crate::fs::remove_link(link).map_err(|e| e.to_string()),
        FsCommand::NotUndoable { .. } => return,
    };
    match result {
        Ok(()) => {
            state.journal.applied(direction, command);
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Err(error) => {
            let message = t("journal_failed").replacen("{}", &error, 1);
            state.dialogs.replace(PopupType::Error(message));
        }
    }
}

/// Queues `job` and remembers which undo/redo it runs.
fn submit(state: &mut AppState, direction: Direction, job: TransferJob, command: FsCommand) {
    state.journal.begin_job(job.id, direction, command);
    crate::fs::transfer::submit::submit_job(state, job);
}

/// Copy/Move of the batch's files, one by one, never overwriting anything.
fn pairs_job(operation: TransferOperation, batch: &FileBatch, context: &AppContext) -> TransferJob {
    let pairs = batch
        .files
        .iter()
        .map(|f| (f.from.clone(), f.to.clone()))
        .collect();
    let options = TransferOptions {
        conflict_resolution: "skip".to_string(),
        follow_symlinks: false,
        skip_symlinks: false,
        filter_mask: None,
        halt_on_error: false,
        delete_to_recycle_bin: false,
        ..crate::fs::transfer::transfer_options_from_settings(&context.config.settings)
    };
    TransferJob::for_pairs(operation, pairs, options).with_prune_dirs(batch.prune_dirs.clone())
}
