//! Reads the Pairee binary out of a verified release archive in memory.
//! Only that single file is read; nothing else is written to disk.

use anyhow::{Context as _, Result};
use std::io::Read as _;

use crate::update::downloader::MAX_ASSET_BYTES;

/// Returns the contents of the first regular file named `name` in a tar.gz
/// buffer.
#[cfg(any(not(target_os = "windows"), test))]
pub fn file_from_tar_gz(archive: &[u8], name: &str) -> Result<Vec<u8>> {
    let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(archive));
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries().context("failed to read archive")? {
        let mut entry = entry.context("corrupt archive entry")?;
        if entry.header().entry_type() != tar::EntryType::Regular {
            continue;
        }
        let path = entry.path().context("invalid archive path")?;
        if path.file_name().is_some_and(|n| n == name) {
            let mut out = Vec::new();
            entry
                .by_ref()
                .take(MAX_ASSET_BYTES)
                .read_to_end(&mut out)
                .context("failed to read binary from archive")?;
            return Ok(out);
        }
    }
    anyhow::bail!("{name} binary not found in extracted archive")
}

/// Reads `pairee.exe` (any directory level, any case) out of a zip buffer.
#[cfg(any(target_os = "windows", test))]
pub fn exe_from_zip(archive: &[u8]) -> Result<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive))
        .context("failed to read zip archive")?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).context("corrupt zip entry")?;
        let is_exe = file
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .is_some_and(|n| n == "pairee.exe");
        if is_exe && file.is_file() {
            let mut out = Vec::new();
            (&mut file)
                .take(MAX_ASSET_BYTES)
                .read_to_end(&mut out)
                .context("failed to read pairee.exe from zip")?;
            return Ok(out);
        }
    }
    anyhow::bail!("pairee.exe not found in archive")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn tar_gz_extracts_only_named_binary() {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, body) in [("pkg/README", &b"readme"[..]), ("pkg/pairee", &b"BIN"[..])] {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, body).unwrap();
        }
        let tar_bytes = builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar_bytes).unwrap();
        let archive = gz.finish().unwrap();

        assert_eq!(file_from_tar_gz(&archive, "pairee").unwrap(), b"BIN");
        assert!(file_from_tar_gz(&archive, "missing").is_err());
    }

    #[test]
    fn zip_extracts_pairee_exe() {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("dist/Pairee.exe", opts).unwrap();
        zip.write_all(b"EXE").unwrap();
        let archive = zip.finish().unwrap().into_inner();

        assert_eq!(exe_from_zip(&archive).unwrap(), b"EXE");
    }

    #[test]
    fn junk_archives_are_errors() {
        assert!(file_from_tar_gz(b"not gzip", "pairee").is_err());
        assert!(exe_from_zip(b"not zip").is_err());
    }
}
