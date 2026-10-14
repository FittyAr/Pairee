//! The one recursive tree walker of the filesystem layer.
//!
//! [`Walker`] lists a tree through the panel source port ([`Vfs`]), so the
//! same code serves local folders, SFTP servers and archives. It is an
//! iterator of [`WalkEntry`]s (parents before children) with three explicit
//! policies:
//!
//! * **links** ([`LinkPolicy`]): skipped, reported without being entered, or
//!   followed with loop detection (a folder whose identity is already on
//!   the path from the root is reported as a [`WalkError::Cycle`]);
//! * **errors**: an unreadable folder is reported as a [`WalkError`] item
//!   and the walk goes on with its siblings; the caller decides whether that
//!   aborts its job (nothing is swallowed);
//! * **cancellation**: a callback polled before each entry; once it
//!   returns `true` the walker yields [`WalkError::Cancelled`] and stops.
//!
//! [`Walker::skip_dir`] prunes the folder just returned (filters, folders
//! the caller handles itself).

mod error;

#[cfg(test)]
pub(crate) mod tests;

pub use error::WalkError;

use crate::fs::file_id::FileIdentity;
use crate::fs::vfs::{Vfs, VfsEntry};
use std::path::{Path, PathBuf};

/// Deepest folder level entered: only pathological trees (or loops the
/// filesystem cannot identify) get this deep.
pub const MAX_WALK_DEPTH: usize = 1024;

/// What the walker does with symbolic links (and Windows junctions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkPolicy {
    /// Links are left out.
    Skip,
    /// Links are reported as entries of their own and never entered.
    #[default]
    Keep,
    /// Links are resolved: a link to a folder is entered (loops detected),
    /// a link to a file is reported with the target's size.
    Follow,
}

/// Options of one walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkOptions {
    pub links: LinkPolicy,
    /// Folders deeper than this are reported as [`WalkError::TooDeep`].
    pub max_depth: usize,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            links: LinkPolicy::Keep,
            max_depth: MAX_WALK_DEPTH,
        }
    }
}

impl WalkOptions {
    /// Default options with `links`.
    pub fn with_links(links: LinkPolicy) -> Self {
        Self {
            links,
            ..Self::default()
        }
    }

    /// Links followed when `follow`, kept otherwise.
    pub fn following(follow: bool) -> Self {
        Self::with_links(if follow {
            LinkPolicy::Follow
        } else {
            LinkPolicy::Keep
        })
    }
}

/// One entry found below the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkEntry {
    /// The entry; for a followed link, `is_dir`, `size` and `modified`
    /// describe the target (`is_symlink` stays set).
    pub entry: VfsEntry,
    /// 1 for the children of the root.
    pub depth: usize,
    /// A folder the walker enters next (unless [`Walker::skip_dir`]).
    pub descends: bool,
    /// A link resolved to its target ([`LinkPolicy::Follow`]).
    pub followed: bool,
}

impl WalkEntry {
    pub fn path(&self) -> &Path {
        &self.entry.path
    }

    /// A folder (a real one, or a followed link to one).
    pub fn is_dir(&self) -> bool {
        self.descends
    }

    /// A link that was reported without being followed.
    pub fn is_kept_link(&self) -> bool {
        self.entry.is_symlink && !self.followed
    }
}

/// A folder waiting to be listed.
struct Pending {
    path: PathBuf,
    depth: usize,
    identity: Option<FileIdentity>,
}

/// A listed folder whose entries are being returned.
struct Frame {
    entries: std::vec::IntoIter<VfsEntry>,
    depth: usize,
    identity: Option<FileIdentity>,
}

/// Depth-first iterator over the tree below a root (the root itself is not
/// returned). See the module documentation for the policies.
pub struct Walker<'a, V: Vfs + ?Sized = dyn Vfs> {
    vfs: &'a V,
    options: WalkOptions,
    cancelled: Option<&'a dyn Fn() -> bool>,
    stack: Vec<Frame>,
    pending: Option<Pending>,
    done: bool,
}

impl<'a, V: Vfs + ?Sized> Walker<'a, V> {
    /// A walk of the tree below `root` on `vfs`.
    pub fn new(vfs: &'a V, root: &Path, options: WalkOptions) -> Self {
        let identity = (options.links == LinkPolicy::Follow)
            .then(|| vfs.identity(root))
            .flatten();
        Self {
            vfs,
            options,
            cancelled: None,
            stack: Vec::new(),
            pending: Some(Pending {
                path: root.to_path_buf(),
                depth: 0,
                identity,
            }),
            done: false,
        }
    }

    /// Stops the walk once `cancelled` returns `true` (polled before each
    /// entry).
    pub fn with_cancel(mut self, cancelled: &'a dyn Fn() -> bool) -> Self {
        self.cancelled = Some(cancelled);
        self
    }

    /// Does not enter the folder returned last.
    pub fn skip_dir(&mut self) {
        self.pending = None;
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.is_some_and(|cancelled| cancelled())
    }

    /// `true` when `identity` is the root or a folder being walked.
    fn on_current_path(&self, identity: &FileIdentity) -> bool {
        self.stack
            .iter()
            .any(|frame| frame.identity.as_ref() == Some(identity))
    }

    /// Lists the pending folder (if any) onto the stack.
    fn open_pending(&mut self) -> Result<(), WalkError> {
        let Some(dir) = self.pending.take() else {
            return Ok(());
        };
        let (entries, skipped) =
            self.vfs
                .list_reporting(&dir.path)
                .map_err(|source| WalkError::Io {
                    path: dir.path.clone(),
                    source,
                })?;
        self.stack.push(Frame {
            entries: entries.into_iter(),
            depth: dir.depth,
            identity: dir.identity,
        });
        // The entries that could be read are still walked.
        match skipped {
            Some(source) => Err(WalkError::Incomplete {
                path: dir.path,
                source,
            }),
            None => Ok(()),
        }
    }

    /// What to return for `entry` found at `depth` (`None`: left out).
    fn classify(
        &mut self,
        mut entry: VfsEntry,
        depth: usize,
    ) -> Option<Result<WalkEntry, WalkError>> {
        let mut followed = false;
        if entry.is_symlink {
            match self.options.links {
                LinkPolicy::Skip => return None,
                LinkPolicy::Keep => return Some(Ok(leaf(entry, depth))),
                LinkPolicy::Follow => match self.vfs.metadata(&entry.path) {
                    Ok(target) => {
                        entry.is_dir = target.is_dir;
                        entry.size = target.size;
                        entry.modified = target.modified;
                        followed = true;
                    }
                    // A broken link: reported as the link it is.
                    Err(_) => return Some(Ok(leaf(entry, depth))),
                },
            }
        }
        if !entry.is_dir {
            return Some(Ok(WalkEntry {
                followed,
                ..leaf(entry, depth)
            }));
        }
        Some(self.enter(entry, depth, followed))
    }

    /// A folder at `depth`: queued for listing unless it loops or is too deep.
    fn enter(
        &mut self,
        entry: VfsEntry,
        depth: usize,
        followed: bool,
    ) -> Result<WalkEntry, WalkError> {
        if depth > self.options.max_depth {
            return Err(WalkError::TooDeep { path: entry.path });
        }
        let identity = (self.options.links == LinkPolicy::Follow)
            .then(|| self.vfs.identity(&entry.path))
            .flatten();
        if identity.as_ref().is_some_and(|id| self.on_current_path(id)) {
            return Err(WalkError::Cycle { path: entry.path });
        }
        self.pending = Some(Pending {
            path: entry.path.clone(),
            depth,
            identity,
        });
        Ok(WalkEntry {
            entry,
            depth,
            descends: true,
            followed,
        })
    }
}

/// An entry the walker does not enter.
fn leaf(entry: VfsEntry, depth: usize) -> WalkEntry {
    WalkEntry {
        entry,
        depth,
        descends: false,
        followed: false,
    }
}

impl<V: Vfs + ?Sized> Iterator for Walker<'_, V> {
    type Item = Result<WalkEntry, WalkError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.done {
                return None;
            }
            if self.is_cancelled() {
                self.done = true;
                return Some(Err(WalkError::Cancelled));
            }
            if let Err(e) = self.open_pending() {
                return Some(Err(e));
            }
            let Some(frame) = self.stack.last_mut() else {
                self.done = true;
                return None;
            };
            let depth = frame.depth + 1;
            let Some(entry) = frame.entries.next() else {
                self.stack.pop();
                continue;
            };
            if let Some(item) = self.classify(entry, depth) {
                return Some(item);
            }
        }
    }
}
