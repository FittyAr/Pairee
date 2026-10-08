//! Where a panel's entries come from: the location half of the panel
//! state (the path within it is `PanelState::current_path`).

use super::{Capabilities, LocalVfs, Vfs};
use crate::fs::archive::{ArchiveVfs, split_archive_path};
use crate::fs::ssh::SharedSshClient;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub enum PanelSource {
    /// The local filesystem.
    #[default]
    Local,
    /// An SFTP server.
    Remote(SharedSshClient),
    /// The inside of a local archive file.
    Archive(Arc<ArchiveVfs>),
}

impl PanelSource {
    /// The adapter serving this source.
    pub fn vfs(&self) -> Arc<dyn Vfs> {
        match self {
            Self::Local => Arc::new(LocalVfs),
            Self::Remote(client) => Arc::new(client.clone()),
            Self::Archive(archive) => archive.clone(),
        }
    }

    /// The adapter that reads `path`, an entry of this source or a local
    /// path outside the archive this source shows.
    pub fn vfs_for(&self, path: &Path) -> Arc<dyn Vfs> {
        match self {
            Self::Archive(archive) if !archive.contains(path) => match archive.parent() {
                Some(parent) => Self::Archive(Arc::clone(parent)).vfs_for(path),
                None => Arc::new(LocalVfs),
            },
            _ => self.vfs(),
        }
    }

    /// The SSH connection of a remote panel (transfer endpoints).
    pub fn ssh(&self) -> Option<&SharedSshClient> {
        match self {
            Self::Remote(client) => Some(client),
            _ => None,
        }
    }

    /// The archive a panel is browsing.
    pub fn archive(&self) -> Option<&ArchiveVfs> {
        match self {
            Self::Archive(archive) => Some(archive),
            _ => None,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }

    pub fn capabilities(&self) -> Capabilities {
        self.vfs().capabilities()
    }

    /// The source for a panel moving to `path`: leaving an archive goes
    /// back to the archive that holds it or to the local disk, a local path
    /// inside an archive file opens that archive, and a path inside an
    /// archive stored in the archive opens that one (read-only). Remote
    /// panels stay remote.
    pub fn locate(&self, path: &Path) -> Self {
        match self {
            Self::Remote(_) => self.clone(),
            Self::Archive(archive) if archive.contains(path) => archive
                .nested_at(path)
                .map_or_else(|| self.clone(), |nested| Self::Archive(Arc::new(nested))),
            Self::Archive(archive) => match archive.parent() {
                Some(parent) => Self::Archive(Arc::clone(parent)).locate(path),
                None => Self::Local.locate(path),
            },
            Self::Local => match split_archive_path(path).and_then(|(a, _)| ArchiveVfs::open(a)) {
                Some(archive) => Self::Archive(Arc::new(archive)).locate(path),
                None => Self::Local,
            },
        }
    }
}
