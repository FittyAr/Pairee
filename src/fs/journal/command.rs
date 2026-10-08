//! Applied filesystem operations (Command) and their inverses.
//!
//! A [`FsCommand`] records what an operation actually did, with enough
//! detail to reverse it: [`FsCommand::inverse`] returns the command that
//! undoes it, which is itself an ordinary command (so redo is the inverse of
//! an applied undo).

use crate::config::localization::t;
use crate::fs::LinkKind;
use crate::fs::multi_rename::Step;
use crate::fs::ssh::SharedSshClient;
use crate::fs::stamp::Stamp;
use std::path::PathBuf;

/// One file moved or copied from `from` to `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStep {
    pub from: PathBuf,
    pub to: PathBuf,
    /// Size and time of the entry the step left: at `to` in a recorded
    /// command, at `from` once [`FsCommand::inverse`] swapped the step.
    pub stamp: Option<Stamp>,
    /// A copy that overwrote an existing file (never deleted by an undo).
    pub replaced: bool,
}

impl FileStep {
    fn swapped(&self) -> Self {
        Self {
            from: self.to.clone(),
            to: self.from.clone(),
            stamp: self.stamp,
            replaced: self.replaced,
        }
    }
}

/// Files of a copy or move, with the folders it created.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileBatch {
    pub files: Vec<FileStep>,
    /// Folders the operation created.
    pub created_dirs: Vec<PathBuf>,
    /// Folders to remove, when empty, after running the batch.
    pub prune_dirs: Vec<PathBuf>,
}

impl FileBatch {
    /// A batch of `files` with no folder bookkeeping.
    pub fn of(files: Vec<FileStep>) -> Self {
        Self {
            files,
            ..Self::default()
        }
    }

    /// The batch that runs `files` and then prunes what `self` created.
    fn reversing(&self, files: Vec<FileStep>) -> Self {
        Self {
            files,
            created_dirs: Vec::new(),
            prune_dirs: self.created_dirs.clone(),
        }
    }
}

/// Why an operation is kept in the journal but cannot be undone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Irreversible {
    /// Permanent delete.
    Delete,
    /// Secure wipe.
    Wipe,
    /// Sent to a trash that cannot be read back on this platform.
    Trash,
    /// Copy, move or delete on an SFTP panel.
    Remote,
    /// Copy into or out of an archive, or delete inside one.
    Archive,
}

/// An applied operation.
#[derive(Debug, Clone)]
pub enum FsCommand {
    /// Renames in the order they ran (local, or SFTP when `ssh` is set).
    Rename {
        steps: Vec<Step>,
        ssh: Option<SharedSshClient>,
    },
    Move(FileBatch),
    Copy(FileBatch),
    /// Copies deleted by undoing a copy (`from` original → `to` copy).
    RemoveCopies(FileBatch),
    Trash {
        paths: Vec<PathBuf>,
    },
    Restore {
        paths: Vec<PathBuf>,
    },
    MakeDir {
        path: PathBuf,
    },
    RemoveDir {
        path: PathBuf,
    },
    Link {
        link: PathBuf,
        target: PathBuf,
        kind: LinkKind,
    },
    Unlink {
        link: PathBuf,
        target: PathBuf,
        kind: LinkKind,
    },
    NotUndoable {
        kind: Irreversible,
        count: usize,
    },
}

impl FsCommand {
    /// The command that reverses this one (`None` when it cannot be undone).
    pub fn inverse(&self) -> Option<Self> {
        Some(match self {
            Self::Rename { steps, ssh } => Self::Rename {
                steps: steps
                    .iter()
                    .rev()
                    .map(|s| Step {
                        from: s.to.clone(),
                        to: s.from.clone(),
                    })
                    .collect(),
                ssh: ssh.clone(),
            },
            Self::Move(batch) => {
                Self::Move(batch.reversing(batch.files.iter().map(FileStep::swapped).collect()))
            }
            Self::Copy(batch) => Self::RemoveCopies(batch.reversing(batch.files.clone())),
            Self::RemoveCopies(batch) => Self::Copy(FileBatch::of(
                batch
                    .files
                    .iter()
                    .map(|f| FileStep {
                        stamp: None,
                        ..f.clone()
                    })
                    .collect(),
            )),
            Self::Trash { paths } => Self::Restore {
                paths: paths.clone(),
            },
            Self::Restore { paths } => Self::Trash {
                paths: paths.clone(),
            },
            Self::MakeDir { path } => Self::RemoveDir { path: path.clone() },
            Self::RemoveDir { path } => Self::MakeDir { path: path.clone() },
            Self::Link { link, target, kind } => Self::Unlink {
                link: link.clone(),
                target: target.clone(),
                kind: *kind,
            },
            Self::Unlink { link, target, kind } => Self::Link {
                link: link.clone(),
                target: target.clone(),
                kind: *kind,
            },
            Self::NotUndoable { .. } => return None,
        })
    }

    pub fn is_undoable(&self) -> bool {
        !matches!(self, Self::NotUndoable { .. })
    }

    /// The entries the command touched: `(path, new path)` for renames,
    /// moves and copies, `(path, None)` otherwise.
    pub fn items(&self) -> Vec<(PathBuf, Option<PathBuf>)> {
        let pairs = |files: &[FileStep]| {
            files
                .iter()
                .map(|f| (f.from.clone(), Some(f.to.clone())))
                .collect()
        };
        match self {
            Self::Rename { steps, .. } => steps
                .iter()
                .map(|s| (s.from.clone(), Some(s.to.clone())))
                .collect(),
            Self::Move(b) | Self::Copy(b) => pairs(&b.files),
            Self::RemoveCopies(b) => b.files.iter().map(|f| (f.to.clone(), None)).collect(),
            Self::Trash { paths } | Self::Restore { paths } => {
                paths.iter().map(|p| (p.clone(), None)).collect()
            }
            Self::MakeDir { path } | Self::RemoveDir { path } => vec![(path.clone(), None)],
            Self::Link { link, target, .. } | Self::Unlink { link, target, .. } => {
                vec![(link.clone(), Some(target.clone()))]
            }
            Self::NotUndoable { .. } => Vec::new(),
        }
    }

    /// Number of entries (for labels).
    pub fn count(&self) -> usize {
        match self {
            Self::NotUndoable { count, .. } => *count,
            other => other.items().len(),
        }
    }

    /// `true` when the command did nothing (it is not journaled).
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Localized verb ("Move", "Send to trash"...).
    pub fn verb(&self) -> String {
        t(match self {
            Self::Rename { .. } => "journal_verb_rename",
            Self::Move(_) => "journal_verb_move",
            Self::Copy(_) => "journal_verb_copy",
            Self::RemoveCopies(_) => "journal_verb_remove_copies",
            Self::Trash { .. } => "journal_verb_trash",
            Self::Restore { .. } => "journal_verb_restore",
            Self::MakeDir { .. } => "journal_verb_mkdir",
            Self::RemoveDir { .. } => "journal_verb_rmdir",
            Self::Link { .. } => "journal_verb_link",
            Self::Unlink { .. } => "journal_verb_unlink",
            Self::NotUndoable { kind, .. } => match kind {
                Irreversible::Delete => "journal_verb_delete",
                Irreversible::Wipe => "journal_verb_wipe",
                Irreversible::Trash => "journal_verb_trash",
                Irreversible::Remote => "journal_verb_remote",
                Irreversible::Archive => "journal_verb_archive",
            },
        })
    }

    /// "Move «a.txt»" or "Move (3 items)".
    pub fn label(&self) -> String {
        let items = self.items();
        let subject = match items.as_slice() {
            [(path, _)] => format!("«{}»", shortened(&crate::fs::file_name_lossy(path))),
            _ => t("journal_items").replacen("{}", &self.count().to_string(), 1),
        };
        format!("{} {}", self.verb(), subject)
    }
}

/// Longest file name shown in a label (menus stay narrow).
const LABEL_NAME_CHARS: usize = 32;

/// `name`, cut to [`LABEL_NAME_CHARS`] with an ellipsis.
fn shortened(name: &str) -> String {
    if name.chars().count() <= LABEL_NAME_CHARS {
        return name.to_string();
    }
    let kept: String = name.chars().take(LABEL_NAME_CHARS - 1).collect();
    format!("{kept}…")
}
