//! Archive detection, extraction, and compression orchestrator.

pub mod external_7z;
pub mod sevenz_ops;
pub mod tar_ops;
pub mod zip_ops;

pub use external_7z::extract_via_external_7z;
pub use sevenz_ops::{extract_7z, list_7z_files};
pub use tar_ops::{extract_tar_gz, list_tar_gz_files};
pub use zip_ops::{compress_zip, extract_zip, list_zip_files};

use anyhow::{Result, anyhow};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use crate::fs::progress::ProgressUpdate;

pub enum ArchiveFormat {
    Zip,
    TarGz,
    SevenZ,
    Rar,
    Iso,
    Unsupported,
}

pub fn detect_format(path: &Path) -> ArchiveFormat {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if ext_str == "zip" {
            return ArchiveFormat::Zip;
        } else if ext_str == "gz" || ext_str == "tgz" {
            return ArchiveFormat::TarGz;
        } else if ext_str == "7z" {
            return ArchiveFormat::SevenZ;
        } else if ext_str == "rar" {
            return ArchiveFormat::Rar;
        } else if ext_str == "iso" {
            return ArchiveFormat::Iso;
        }
    }
    ArchiveFormat::Unsupported
}

pub fn extract_archive(
    archive_path: &Path,
    dest_dir: &Path,
    tx: &mpsc::Sender<ProgressUpdate>,
    cancel: &AtomicBool,
) -> Result<()> {
    match detect_format(archive_path) {
        ArchiveFormat::Zip => extract_zip(archive_path, dest_dir, tx, cancel),
        ArchiveFormat::TarGz => extract_tar_gz(archive_path, dest_dir, tx, cancel),
        ArchiveFormat::SevenZ => extract_7z(archive_path, dest_dir, tx, cancel),
        ArchiveFormat::Rar | ArchiveFormat::Iso => {
            extract_via_external_7z(archive_path, dest_dir, tx, cancel)
        }
        ArchiveFormat::Unsupported => Err(anyhow!("Unsupported archive format")),
    }
}

pub fn list_archive_files(path: &Path) -> Result<Vec<String>> {
    match detect_format(path) {
        ArchiveFormat::Zip => list_zip_files(path),
        ArchiveFormat::TarGz => list_tar_gz_files(path),
        ArchiveFormat::SevenZ => list_7z_files(path),
        _ => Err(anyhow!(
            "Unsupported archive format or listing not supported"
        )),
    }
}
