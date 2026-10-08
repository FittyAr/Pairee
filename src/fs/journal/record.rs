//! Journal entries of finished Transfer Engine jobs.

use super::command::{FileBatch, FileStep, FsCommand, Irreversible};
use super::trash::RESTORE_SUPPORTED;
use crate::fs::multi_rename::Step;
use crate::fs::ssh::SharedSshClient;
use crate::fs::transfer::job::{TransferJob, TransferOperation, TransferResults};
use std::collections::HashSet;
use std::path::PathBuf;

/// What a finished job did, from its `results`. `executed` is the command
/// the job ran for an undo/redo (`None` for a user operation). Jobs that are
/// not journaled (compress, extract, apply command, copies in and out of
/// archives) give `None`.
pub fn from_transfer(
    job: &TransferJob,
    results: &TransferResults,
    executed: Option<&FsCommand>,
) -> Option<FsCommand> {
    if crate::fs::transfer::backend::archive_vfs::plan(job).is_some() {
        return None;
    }
    if let Some(client) = same_server_move(job) {
        // A move within one SFTP server is a series of renames.
        let command = FsCommand::Rename {
            steps: rename_steps(results),
            ssh: Some(client),
        };
        return (!command.is_empty()).then_some(command);
    }
    let done: Vec<PathBuf> = results
        .completed_files
        .iter()
        .map(|f| f.src.clone())
        .collect();
    let not_undoable = |kind| FsCommand::NotUndoable {
        kind,
        count: done.len(),
    };
    let command = match job.operation {
        TransferOperation::Copy | TransferOperation::Move | TransferOperation::Delete
            if job.ssh.is_some() =>
        {
            not_undoable(Irreversible::Remote)
        }
        TransferOperation::Copy => FsCommand::Copy(batch(results)),
        TransferOperation::Move => FsCommand::Move(batch(results)),
        TransferOperation::Delete => match executed {
            Some(FsCommand::RemoveCopies(removed)) => {
                FsCommand::RemoveCopies(only_removed(removed, &done))
            }
            _ if !job.options.delete_to_recycle_bin => not_undoable(Irreversible::Delete),
            _ if !RESTORE_SUPPORTED => not_undoable(Irreversible::Trash),
            _ => FsCommand::Trash { paths: done },
        },
        TransferOperation::Wipe => not_undoable(Irreversible::Wipe),
        TransferOperation::Restore => FsCommand::Restore { paths: done },
        TransferOperation::Compress
        | TransferOperation::Extract
        | TransferOperation::ApplyCommand => return None,
    };
    (!command.is_empty()).then_some(command)
}

/// The server of a move whose source and destination are on the same SFTP
/// connection (run as renames, see `fast_remote_rename`).
fn same_server_move(job: &TransferJob) -> Option<SharedSshClient> {
    let ssh = job.ssh.as_ref()?;
    match (&ssh.src, &ssh.dst) {
        (Some(src), Some(dst))
            if job.operation == TransferOperation::Move && src.is_same_server(dst) =>
        {
            Some(src.clone())
        }
        _ => None,
    }
}

/// The renames a job ran, one per completed entry.
pub(super) fn rename_steps(results: &TransferResults) -> Vec<Step> {
    results
        .completed_files
        .iter()
        .map(|f| Step {
            from: f.src.clone(),
            to: f.dst.clone(),
        })
        .collect()
}

/// The files and folders a copy/move created.
fn batch(results: &TransferResults) -> FileBatch {
    FileBatch {
        files: results
            .completed_files
            .iter()
            .map(|f| FileStep {
                from: f.src.clone(),
                to: f.dst.clone(),
                stamp: f.dst_stamp,
                replaced: f.replaced,
            })
            .collect(),
        created_dirs: results.created_dirs.clone(),
        prune_dirs: Vec::new(),
    }
}

/// The copies of `removed` that were actually deleted (`done`).
fn only_removed(removed: &FileBatch, done: &[PathBuf]) -> FileBatch {
    let done: HashSet<&PathBuf> = done.iter().collect();
    FileBatch::of(
        removed
            .files
            .iter()
            .filter(|f| done.contains(&f.to))
            .cloned()
            .collect(),
    )
}
