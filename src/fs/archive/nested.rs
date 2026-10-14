//! Where an [`super::ArchiveVfs`] reads its archive from: a local file, or
//! (for an archive inside another one) a private temporary copy extracted
//! from the containing archive the first time it is needed, so the panel
//! thread never waits for it. Nested archives are read-only.

use super::edit::PrivateTempFile;
use super::vfs::ArchiveVfs;
use crate::config::localization::t_fill;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Largest inner archive extracted for browsing.
pub const MAX_NESTED_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

/// The archive file behind a panel's archive source.
#[derive(Debug)]
pub enum ArchiveData {
    /// An archive file on the local disk.
    Local(PathBuf),
    /// The entry `entry` of `parent`, extracted on first use.
    Nested {
        parent: Arc<ArchiveVfs>,
        entry: PathBuf,
        extracted: OnceLock<Result<PrivateTempFile, String>>,
    },
}

impl ArchiveData {
    pub fn nested(parent: Arc<ArchiveVfs>, entry: PathBuf) -> Self {
        Self::Nested {
            parent,
            entry,
            extracted: OnceLock::new(),
        }
    }

    /// The local file to read (extracting a nested archive first).
    pub fn file(&self) -> io::Result<PathBuf> {
        match self {
            Self::Local(path) => Ok(path.clone()),
            Self::Nested {
                parent,
                entry,
                extracted,
            } => extracted
                .get_or_init(|| extract(parent, entry).map_err(|e| e.to_string()))
                .as_ref()
                .map(|x| x.path().to_path_buf())
                .map_err(|message| io::Error::other(message.clone())),
        }
    }

    /// The containing archive of a nested one.
    pub fn parent(&self) -> Option<&Arc<ArchiveVfs>> {
        match self {
            Self::Local(_) => None,
            Self::Nested { parent, .. } => Some(parent),
        }
    }
}

/// Copies `entry` of `parent` into a new private temporary file, refusing
/// archives above [`MAX_NESTED_ARCHIVE_BYTES`].
fn extract(parent: &ArchiveVfs, entry: &Path) -> io::Result<PrivateTempFile> {
    let size = crate::fs::vfs::Vfs::stat(parent, entry)?.size;
    if size > MAX_NESTED_ARCHIVE_BYTES {
        return Err(io::Error::other(too_large(size)));
    }
    // The file keeps the entry's name: the format is told by its extension.
    let extracted = PrivateTempFile::new(&crate::fs::file_name_lossy(entry))?;
    let mut out = std::fs::File::create(extracted.path())?;
    let written = parent.copy_entry(entry, &mut out, MAX_NESTED_ARCHIVE_BYTES + 1)?;
    if written > MAX_NESTED_ARCHIVE_BYTES {
        return Err(io::Error::other(too_large(written)));
    }
    Ok(extracted)
}

fn too_large(size: u64) -> String {
    t_fill(
        "archive_nested_too_large",
        &[
            &bytesize::ByteSize::b(size).to_string(),
            &bytesize::ByteSize::b(MAX_NESTED_ARCHIVE_BYTES).to_string(),
        ],
    )
}
