//! In-place zip editing: the archive is rewritten to a temporary file next
//! to it (kept entries are copied raw, without recompressing) and then
//! atomically renamed over the original, so a failure never leaves a
//! half-written archive behind.

use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

use super::safe_extract::ExtractGuard;

/// Content of an entry added to the archive.
#[derive(Debug, Clone)]
pub enum ZipSource {
    Dir,
    Bytes(Vec<u8>),
    /// A local file, streamed into the archive.
    File(PathBuf),
}

/// Changes applied by one [`rewrite_zip`]. Paths are relative to the
/// archive root.
#[derive(Debug, Clone, Default)]
pub struct ZipEdit {
    /// Entries removed with everything below them.
    pub remove: Vec<PathBuf>,
    /// Entries added; an existing entry with the same path is replaced.
    pub add: Vec<(PathBuf, ZipSource)>,
}

/// Entries this large need the Zip64 format.
const ZIP64_THRESHOLD: u64 = u32::MAX as u64;

/// Options of a deflated entry of `size` bytes (Zip64 when it needs it:
/// without it the writer fails on entries over 4 GiB).
pub(super) fn file_options(size: u64) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(size >= ZIP64_THRESHOLD)
}

/// `/`-separated entry name of `rel` (folders end with `/`).
fn entry_name(rel: &Path, dir: bool) -> String {
    let mut name = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    if dir {
        name.push('/');
    }
    name
}

/// Applies `edit` to `archive`. `on_added` runs after each added entry
/// (progress, cancellation: an error aborts and keeps the original).
pub fn rewrite_zip(
    archive: &Path,
    edit: &ZipEdit,
    on_added: &mut dyn FnMut(&Path) -> io::Result<()>,
) -> Result<()> {
    let dir = archive.parent().unwrap_or(Path::new("."));
    let mut old = ZipArchive::new(fs::File::open(archive)?)?;
    let mut out = zip::ZipWriter::new(tempfile::NamedTempFile::new_in(dir)?);
    let replaced: HashSet<&Path> = edit.add.iter().map(|(rel, _)| rel.as_path()).collect();
    for i in 0..old.len() {
        let file = old.by_index_raw(i)?;
        let dropped = ExtractGuard::sanitize(&file.name()?).is_ok_and(|rel| {
            replaced.contains(rel.as_path()) || edit.remove.iter().any(|r| rel.starts_with(r))
        });
        if !dropped {
            out.raw_copy_file(file)?;
        }
    }
    for (rel, source) in &edit.add {
        match source {
            ZipSource::Dir => out.add_directory(entry_name(rel, true), file_options(0))?,
            ZipSource::Bytes(bytes) => {
                out.start_file(entry_name(rel, false), file_options(bytes.len() as u64))?;
                out.write_all(bytes)?;
            }
            ZipSource::File(path) => {
                let mut file = fs::File::open(path)?;
                let size = file.metadata()?.len();
                out.start_file(entry_name(rel, false), file_options(size))?;
                io::copy(&mut file, &mut out)?;
            }
        }
        on_added(rel)?;
    }
    let tmp = out.finish()?;
    // The temporary file is private (0600): give it the archive's mode.
    tmp.as_file()
        .set_permissions(fs::metadata(archive)?.permissions())?;
    // Windows cannot replace a file that is still open.
    drop(old);
    tmp.persist(archive)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_one(archive: &Path) {
        super::super::test_fixtures::write_zip(archive, &[("keep.txt", b"k")]);
        let edit = ZipEdit {
            add: vec![(PathBuf::from("new.txt"), ZipSource::Bytes(b"n".to_vec()))],
            ..ZipEdit::default()
        };
        rewrite_zip(archive, &edit, &mut |_| Ok(())).unwrap();
    }

    #[test]
    fn rewriting_adds_and_keeps_entries() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.zip");
        add_one(&archive);
        let zip = ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
        let mut names: Vec<_> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["keep.txt", "new.txt"]);
    }

    /// Regression (FS-14): the rewritten archive kept the 0600 mode of its
    /// temporary file.
    #[cfg(unix)]
    #[test]
    fn rewriting_keeps_the_archive_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("a.zip");
        super::super::test_fixtures::write_zip(&archive, &[("old.txt", b"o")]);
        fs::set_permissions(&archive, fs::Permissions::from_mode(0o644)).unwrap();
        let edit = ZipEdit {
            add: vec![(PathBuf::from("new.txt"), ZipSource::Bytes(b"n".to_vec()))],
            ..ZipEdit::default()
        };
        rewrite_zip(&archive, &edit, &mut |_| Ok(())).unwrap();
        let mode = fs::metadata(&archive).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);
    }
}
