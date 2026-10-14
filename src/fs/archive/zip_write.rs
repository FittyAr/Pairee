//! Zip rewriting for [`super::edit`]: kept entries are copied raw, without
//! recompressing; added files keep their permissions and time.

use anyhow::Result;
use std::fs;
use std::io;
use std::path::Path;
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

use super::edit::{ArchiveEdit, EntrySource, OnAdded, entry_name, replace_archive};

/// Entries this large need the Zip64 format.
const ZIP64_THRESHOLD: u64 = u32::MAX as u64;

/// Options of a deflated entry of `size` bytes (Zip64 when it needs it:
/// without it the writer fails on entries over 4 GiB).
pub(super) fn file_options(size: u64) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(size >= ZIP64_THRESHOLD)
}

/// The options of an entry taken from the local `meta`: its time and,
/// on Unix, its permission bits.
fn options_from(meta: &fs::Metadata) -> SimpleFileOptions {
    let mut options = file_options(meta.len());
    if let Some(time) = meta.modified().ok().and_then(zip_time) {
        options = options.last_modified_time(time);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        options = options.unix_permissions(meta.permissions().mode() & 0o7777);
    }
    options
}

/// `time` as a zip time stamp (stored as UTC, as the reader reads it).
fn zip_time(time: std::time::SystemTime) -> Option<zip::DateTime> {
    use chrono::{Datelike, Timelike};
    let t = chrono::DateTime::<chrono::Utc>::from(time);
    let narrow = |v: u32| u8::try_from(v).ok();
    zip::DateTime::from_date_and_time(
        u16::try_from(t.year()).ok()?,
        narrow(t.month())?,
        narrow(t.day())?,
        narrow(t.hour())?,
        narrow(t.minute())?,
        narrow(t.second())?,
    )
    .ok()
}

/// Applies `edit` to the zip file `archive`.
pub fn rewrite_zip(archive: &Path, edit: &ArchiveEdit, on_added: &mut OnAdded) -> Result<()> {
    replace_archive(archive, |tmp| {
        let mut old = ZipArchive::new(fs::File::open(archive)?)?;
        let mut out = zip::ZipWriter::new(tmp);
        let dropper = edit.dropper();
        for i in 0..old.len() {
            let file = old.by_index_raw(i)?;
            if !dropper.drops(&file.name()?) {
                out.raw_copy_file(file)?;
            }
        }
        for (rel, source) in &edit.add {
            match source {
                EntrySource::Dir => out.add_directory(entry_name(rel, true), file_options(0))?,
                EntrySource::File(path) => {
                    let meta = fs::metadata(path)?;
                    if meta.is_dir() {
                        out.add_directory(entry_name(rel, true), options_from(&meta))?;
                    } else {
                        out.start_file(entry_name(rel, false), options_from(&meta))?;
                        io::copy(&mut fs::File::open(path)?, &mut out)?;
                    }
                }
            }
            on_added(rel)?;
        }
        out.finish()?;
        Ok(())
    })
}
