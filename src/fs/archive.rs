// Legacy archive helpers kept around for the UI's
// archive browser (ui/quickview.rs,
// input_popup/archive_commands.rs). The actual
// compress/extract engine moved to
// fs/transfer/pipeline.rs; this module just
// provides the small utilities those UI surfaces
// still need (format detection + entry listing).
//
// A10 left the entry listing on the legacy code
// path because the new pipeline is write-only:
// it streams the archive contents to disk, but
// does not hand the UI a list of "what's inside".
// The next cleanup pass should make
// extract_pipeline accept a callback (or expose
// the entry list before extraction) so this whole
// file can be deleted.

use anyhow::{Result, anyhow};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// List the entries inside an archive. The legacy
/// implementation emitted ProgressUpdates for
/// the UI; the new code path doesn't need that
/// because the operation is small and runs
/// synchronously. We keep the function for
/// ui/quickview.rs which only needs the names.
pub fn list_archive_files(path: &Path) -> Result<Vec<String>> {
    match detect_format(path) {
        ArchiveFormat::Zip => list_zip(path),
        ArchiveFormat::TarGz => list_targz(path),
        ArchiveFormat::SevenZ => list_7z(path),
        ArchiveFormat::Rar | ArchiveFormat::Iso => Err(anyhow!(
            "Listing entries for this format requires an external tool"
        )),
        ArchiveFormat::Unsupported => Err(anyhow!("Unsupported archive format")),
    }
}

fn list_zip(path: &Path) -> Result<Vec<String>> {
    use std::fs::File;
    use std::io::Read;
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut names = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        names.push(entry.name().to_string());
    }
    Ok(names)
}

fn list_targz(path: &Path) -> Result<Vec<String>> {
    use flate2::read::GzDecoder;
    use std::fs::File;
    use std::io::Read;
    let bytes = {
        let mut buf = Vec::new();
        File::open(path)?.read_to_end(&mut buf)?;
        buf
    };
    let gz = GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(gz);
    let mut names = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        if let Ok(p) = entry.path() {
            names.push(p.to_string_lossy().into_owned());
        }
    }
    Ok(names)
}

fn list_7z(path: &Path) -> Result<Vec<String>> {
    use std::fs::File;
    use std::io::Read;
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    let cursor = std::io::Cursor::new(bytes);
    let mut names: Vec<String> = Vec::new();
    sevenz_rust::decompress_with_extract_fn(cursor, path, |entry, _reader, _dest| {
        names.push(entry.name().to_string());
        Ok(true)
    })?;
    Ok(names)
}
