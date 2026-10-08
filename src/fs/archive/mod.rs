//! Archive detection, extraction, browsing and compression.
//!
//! Native formats (zip, tar, tar.gz, 7z) implement [`ArchiveReader`]
//! (Strategy); extraction ([`extract`]), the folder tree used to browse an
//! archive in a panel ([`ArchiveVfs`]) and listings all run on it. Rar and
//! ISO images are extracted by an external 7-Zip.

pub mod external_7z;
pub mod extract;
pub mod format;
pub mod index;
pub mod safe_extract;
pub mod sevenz_ops;
pub mod tar_ops;
#[cfg(test)]
pub(crate) mod test_fixtures;
#[cfg(test)]
mod tests;
pub mod vfs;
pub mod zip_ops;
pub mod zip_write;

pub use external_7z::extract_via_external_7z;
pub use format::ArchiveReader;
pub use vfs::ArchiveVfs;
pub use zip_ops::compress_zip;
pub use zip_write::{ZipEdit, ZipSource};

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use crate::fs::progress::ProgressUpdate;
use extract::{everything, extract_entries};

static ZIP: zip_ops::ZipReader = zip_ops::ZipReader;
static TAR: tar_ops::TarReader = tar_ops::TarReader { gz: false };
static TAR_GZ: tar_ops::TarReader = tar_ops::TarReader { gz: true };
static SEVEN_Z: sevenz_ops::SevenZReader = sevenz_ops::SevenZReader;

pub enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    SevenZ,
    Rar,
    Iso,
    Unsupported,
}

impl ArchiveFormat {
    /// The native reader of this format (`None` for external-only formats).
    pub fn reader(&self) -> Option<&'static dyn ArchiveReader> {
        match self {
            Self::Zip => Some(&ZIP),
            Self::Tar => Some(&TAR),
            Self::TarGz => Some(&TAR_GZ),
            Self::SevenZ => Some(&SEVEN_Z),
            Self::Rar | Self::Iso | Self::Unsupported => None,
        }
    }

    /// Short format name shown in previews.
    pub fn label(&self) -> Option<&'static str> {
        match self {
            Self::Zip => Some("ZIP"),
            Self::Tar => Some("TAR"),
            Self::TarGz => Some("TarGz"),
            Self::SevenZ => Some("7Z"),
            Self::Rar | Self::Iso | Self::Unsupported => None,
        }
    }
}

pub fn detect_format(path: &Path) -> ArchiveFormat {
    let Some(ext) = path.extension() else {
        return ArchiveFormat::Unsupported;
    };
    match ext.to_string_lossy().to_lowercase().as_str() {
        "zip" => ArchiveFormat::Zip,
        "tar" => ArchiveFormat::Tar,
        "gz" | "tgz" => ArchiveFormat::TarGz,
        "7z" => ArchiveFormat::SevenZ,
        "rar" => ArchiveFormat::Rar,
        "iso" => ArchiveFormat::Iso,
        _ => ArchiveFormat::Unsupported,
    }
}

/// The reader of an archive that can be browsed as a folder. A bare
/// `.gz` is a single compressed file, not a folder: only `.tar.gz` / `.tgz`.
pub fn browsable_reader(path: &Path) -> Option<&'static dyn ArchiveReader> {
    let format = detect_format(path);
    let name = crate::fs::file_name_lossy(path).to_lowercase();
    if matches!(format, ArchiveFormat::TarGz)
        && !(name.ends_with(".tar.gz") || name.ends_with(".tgz"))
    {
        return None;
    }
    format.reader()
}

/// `path` names an archive that can be browsed as a folder (by extension).
pub fn is_browsable(path: &Path) -> bool {
    browsable_reader(path).is_some()
}

/// Splits `path` into the archive file it lies in and the path inside it,
/// when one of its components (or `path` itself) is a browsable archive
/// file. Only names with an archive extension are checked on disk.
pub fn split_archive_path(path: &Path) -> Option<(PathBuf, PathBuf)> {
    path.ancestors()
        .filter(|a| is_browsable(a))
        .find(|a| a.is_file())
        .map(|archive| {
            let inner = path.strip_prefix(archive).unwrap_or(Path::new(""));
            (archive.to_path_buf(), inner.to_path_buf())
        })
}

pub fn extract_archive(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let format = detect_format(archive_path);
    match format.reader() {
        Some(reader) => extract_entries(reader, archive_path, dest_dir, &everything, tx, cancel),
        None if matches!(format, ArchiveFormat::Rar | ArchiveFormat::Iso) => {
            extract_via_external_7z(archive_path, dest_dir, tx, cancel)
        }
        None => Err(anyhow!("Unsupported archive format")),
    }
}

/// Extracts `selected` (paths inside `archive`, files or folders with
/// everything below them) into `dest_dir`, each relative to the archive
/// folder `base` (the folder the panel shows).
pub fn extract_selected(
    archive: &Path,
    base: &Path,
    selected: &[PathBuf],
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    let reader = browsable_reader(archive).ok_or_else(|| anyhow!("Unsupported archive format"))?;
    let select = |rel: &Path| {
        selected
            .iter()
            .any(|s| rel.starts_with(s))
            .then(|| rel.strip_prefix(base).ok().map(Path::to_path_buf))
            .flatten()
    };
    extract_entries(reader, archive, dest_dir, &select, tx, cancel)
}

pub fn list_archive_files(path: &Path) -> Result<Vec<String>> {
    let reader = browsable_reader(path)
        .ok_or_else(|| anyhow!("Unsupported archive format or listing not supported"))?;
    Ok(reader.entries(path)?.into_iter().map(|e| e.name).collect())
}
