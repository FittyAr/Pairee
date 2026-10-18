//! Archive detection, extraction, browsing and compression.
//!
//! Every format implements [`ArchiveReader`] (Strategy); extraction
//! ([`extract`]), the folder tree used to browse an archive in a panel
//! ([`ArchiveVfs`]), listings and tests all run on it. Zip, tar (plain,
//! gzip, bzip2, xz) and 7z are read natively; RAR, ISO images and the other
//! formats 7-Zip knows are read through an external 7-Zip
//! ([`external_7z`]), which also reads the 7z archives the native reader
//! cannot decode ([`fallback`]).

pub mod edit;
pub mod external_7z;
pub mod extract;
pub mod fallback;
pub mod format;
pub mod index;
pub mod nested;
pub mod safe_extract;
pub mod sevenz_ops;
pub mod sevenz_write;
pub mod tar_ops;
pub mod tar_write;
#[cfg(test)]
pub(crate) mod test_fixtures;
#[cfg(test)]
mod tests;
pub mod vfs;
#[cfg(test)]
mod write_tests;
pub mod zip_ops;
pub mod zip_write;

pub use edit::{ArchiveEdit, EntrySource};
pub use format::ArchiveReader;
pub use vfs::ArchiveVfs;
pub use zip_ops::compress_zip;

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use crate::config::localization::t;
use crate::fs::progress::ArchiveReport;
use extract::extract_entries;
pub use extract::{Existing, Selection};

static ZIP: zip_ops::ZipReader = zip_ops::ZipReader;
static TAR: tar_ops::TarReader = tar_ops::TarReader {
    compression: tar_ops::TarCompression::None,
};
static TAR_GZ: tar_ops::TarReader = tar_ops::TarReader {
    compression: tar_ops::TarCompression::Gzip,
};
static TAR_BZ2: tar_ops::TarReader = tar_ops::TarReader {
    compression: tar_ops::TarCompression::Bzip2,
};
static TAR_XZ: tar_ops::TarReader = tar_ops::TarReader {
    compression: tar_ops::TarCompression::Xz,
};
static SEVEN_Z_NATIVE: sevenz_ops::SevenZReader = sevenz_ops::SevenZReader;
static EXTERNAL: external_7z::External7zReader = external_7z::External7zReader;
static SEVEN_Z: fallback::Fallback = fallback::Fallback {
    native: &SEVEN_Z_NATIVE,
    external: &EXTERNAL,
};

/// Formats read only through an external 7-Zip: extension and the name
/// shown in previews. The full 7-Zip reads them all; the reduced `7za`
/// only CAB (see [`crate::fs::external_tools::SevenZipTool`]).
const EXTERNAL_FORMATS: &[(&str, &str)] = &[
    ("rar", "RAR"),
    ("iso", "ISO"),
    ("cab", "CAB"),
    ("wim", "WIM"),
    ("swm", "WIM"),
    ("esd", "WIM"),
    ("arj", "ARJ"),
    ("lzh", "LZH"),
    ("lha", "LZH"),
    ("cpio", "CPIO"),
    ("rpm", "RPM"),
    ("deb", "DEB"),
    ("xar", "XAR"),
    ("dmg", "DMG"),
    ("vhd", "VHD"),
    ("vhdx", "VHDX"),
    ("squashfs", "SquashFS"),
    ("cramfs", "CramFS"),
    ("chm", "CHM"),
    ("udf", "UDF"),
    ("zst", "Zstandard"),
    ("tzst", "Zstandard"),
    ("lzma", "LZMA"),
    // The first part of a split archive (`backup.7z.001`).
    ("001", "Split"),
];

pub enum ArchiveFormat {
    Zip,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    SevenZ,
    /// Read through an external 7-Zip; the preview label.
    External(&'static str),
    Unsupported,
}

impl ArchiveFormat {
    /// The reader of this format.
    pub fn reader(&self) -> Option<&'static dyn ArchiveReader> {
        match self {
            Self::Zip => Some(&ZIP),
            Self::Tar => Some(&TAR),
            Self::TarGz => Some(&TAR_GZ),
            Self::TarBz2 => Some(&TAR_BZ2),
            Self::TarXz => Some(&TAR_XZ),
            Self::SevenZ => Some(&SEVEN_Z),
            Self::External(_) => Some(&EXTERNAL),
            Self::Unsupported => None,
        }
    }

    /// Short format name shown in previews.
    pub fn label(&self) -> Option<&'static str> {
        match self {
            Self::Zip => Some("ZIP"),
            Self::Tar => Some("TAR"),
            Self::TarGz => Some("TarGz"),
            Self::TarBz2 => Some("TarBz2"),
            Self::TarXz => Some("TarXz"),
            Self::SevenZ => Some("7Z"),
            Self::External(label) => Some(label),
            Self::Unsupported => None,
        }
    }
}

pub fn detect_format(path: &Path) -> ArchiveFormat {
    let Some(ext) = path.extension() else {
        return ArchiveFormat::Unsupported;
    };
    let ext = ext.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "zip" => ArchiveFormat::Zip,
        "tar" => ArchiveFormat::Tar,
        "gz" | "tgz" => ArchiveFormat::TarGz,
        "bz2" | "tbz2" | "tbz" => ArchiveFormat::TarBz2,
        "xz" | "txz" => ArchiveFormat::TarXz,
        "7z" => ArchiveFormat::SevenZ,
        other => EXTERNAL_FORMATS
            .iter()
            .find(|(e, _)| *e == other)
            .map_or(ArchiveFormat::Unsupported, |(_, label)| {
                ArchiveFormat::External(label)
            }),
    }
}

/// The reader of an archive that can be browsed as a folder. A bare `.gz`,
/// `.bz2` or `.xz` is a single compressed file, not a tar (only the
/// `.tar.*` names and their short forms `.tgz`, `.tbz2`, `.tbz`, `.txz`
/// are): 7-Zip shows it as a folder holding that one file.
pub fn browsable_reader(path: &Path) -> Option<&'static dyn ArchiveReader> {
    let format = detect_format(path);
    let name = crate::fs::file_name_lossy(path).to_lowercase();
    let compressed_tar = |exts: &[&str]| exts.iter().any(|ext| name.ends_with(ext));
    let is_tar = match format {
        ArchiveFormat::TarGz => compressed_tar(&[".tar.gz", ".tgz"]),
        ArchiveFormat::TarBz2 => compressed_tar(&[".tar.bz2", ".tbz2", ".tbz"]),
        ArchiveFormat::TarXz => compressed_tar(&[".tar.xz", ".txz"]),
        _ => true,
    };
    if !is_tar {
        return Some(&EXTERNAL);
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

/// Extracts the whole archive (tests; jobs go through the transfer engine).
#[cfg(test)]
pub fn extract_archive(
    archive_path: &Path,
    dest_dir: &Path,
    report: &mut ArchiveReport,
) -> Result<()> {
    extract_selected(
        archive_path,
        &Selection::whole(),
        dest_dir,
        &Existing::default(),
        report,
    )
}

/// Extracts the entries `selection` takes into `dest_dir`; `existing` says
/// what to do with destinations that are already there.
pub fn extract_selected(
    archive: &Path,
    selection: &Selection,
    dest_dir: &Path,
    existing: &Existing,
    report: &mut ArchiveReport,
) -> Result<()> {
    let reader = reader_of(archive)?;
    extract_entries(reader, archive, dest_dir, selection, existing, report)
}

/// The reader of a browsable `archive`, or why there is none.
pub fn reader_of(archive: &Path) -> Result<&'static dyn ArchiveReader> {
    browsable_reader(archive).ok_or_else(|| anyhow!(t("archive_unsupported_format")))
}

/// Decompresses the whole archive to check it (see [`ArchiveReader::test`]).
pub fn test_archive(path: &Path) -> Result<()> {
    reader_of(path)?.test(path)
}

pub fn list_archive_files(path: &Path) -> Result<Vec<String>> {
    Ok(reader_of(path)?
        .entries(path)?
        .into_iter()
        .map(|e| e.name)
        .collect())
}
