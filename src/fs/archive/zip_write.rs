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
        let dropped = ExtractGuard::sanitize(file.name()).is_ok_and(|rel| {
            replaced.contains(rel.as_path()) || edit.remove.iter().any(|r| rel.starts_with(r))
        });
        if !dropped {
            out.raw_copy_file(file)?;
        }
    }
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (rel, source) in &edit.add {
        match source {
            ZipSource::Dir => out.add_directory(entry_name(rel, true), options)?,
            ZipSource::Bytes(bytes) => {
                out.start_file(entry_name(rel, false), options)?;
                out.write_all(bytes)?;
            }
            ZipSource::File(path) => {
                out.start_file(entry_name(rel, false), options)?;
                io::copy(&mut fs::File::open(path)?, &mut out)?;
            }
        }
        on_added(rel)?;
    }
    let tmp = out.finish()?;
    // Windows cannot replace a file that is still open.
    drop(old);
    tmp.persist(archive)?;
    Ok(())
}
