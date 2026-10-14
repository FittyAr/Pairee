//! Tar rewriting for [`super::edit`] (plain, gzip, bzip2 and xz): the old
//! archive is streamed through its decoder, kept entries are written again
//! with their headers (permissions, owner, times, link targets) and the
//! result is compressed as the original was. Added files keep their
//! permissions and time. PAX extended attributes of kept entries are not
//! carried over.

use anyhow::Result;
use std::borrow::Cow;
use std::fs;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;
use std::time::SystemTime;
use tar::{Builder, EntryType, Header, HeaderMode};

use super::edit::{ArchiveEdit, EntrySource, OnAdded, entry_name, replace_archive};
use super::tar_ops::{TarCompression, TarReader};

/// Mode of added folders without a source folder.
const DIR_MODE: u32 = 0o755;
/// xz preset of rewritten archives (the `xz` command's default).
const XZ_PRESET: u32 = 6;

/// The compressor of a rewritten archive.
enum Encoder<W: Write> {
    Plain(W),
    Gzip(flate2::write::GzEncoder<W>),
    Bzip2(bzip2::write::BzEncoder<W>),
    Xz(Box<lzma_rust2::XzWriter<W>>),
}

impl<W: Write> Encoder<W> {
    fn new(compression: TarCompression, out: W) -> io::Result<Self> {
        Ok(match compression {
            TarCompression::None => Self::Plain(out),
            TarCompression::Gzip => Self::Gzip(flate2::write::GzEncoder::new(
                out,
                flate2::Compression::default(),
            )),
            TarCompression::Bzip2 => Self::Bzip2(bzip2::write::BzEncoder::new(
                out,
                bzip2::Compression::default(),
            )),
            TarCompression::Xz => {
                let options = lzma_rust2::XzOptions::with_preset(XZ_PRESET);
                Self::Xz(Box::new(lzma_rust2::XzWriter::new(out, options)?))
            }
        })
    }

    /// Writes the end of the compressed stream; returns the output.
    fn finish(self) -> io::Result<W> {
        match self {
            Self::Plain(out) => Ok(out),
            Self::Gzip(e) => e.finish(),
            Self::Bzip2(e) => e.finish(),
            Self::Xz(e) => e.finish(),
        }
    }
}

impl<W: Write> Write for Encoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Plain(w) => w.write(buf),
            Self::Gzip(w) => w.write(buf),
            Self::Bzip2(w) => w.write(buf),
            Self::Xz(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Plain(w) => w.flush(),
            Self::Gzip(w) => w.flush(),
            Self::Bzip2(w) => w.flush(),
            Self::Xz(w) => w.flush(),
        }
    }
}

/// Applies `edit` to the tar archive `archive` read by `reader`.
pub fn rewrite_tar(
    reader: &TarReader,
    archive: &Path,
    edit: &ArchiveEdit,
    on_added: &mut OnAdded,
) -> Result<()> {
    replace_archive(archive, |tmp| {
        let mut old = reader.open(archive)?;
        let encoder = Encoder::new(reader.compression, BufWriter::new(tmp))?;
        let mut out = Builder::new(encoder);
        out.follow_symlinks(false);
        let dropper = edit.dropper();
        for entry in old.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.into_owned();
            if !dropper.drops(&path.to_string_lossy()) {
                copy_entry(&mut out, &mut entry, &path)?;
            }
        }
        for (rel, source) in &edit.add {
            add_entry(&mut out, rel, source)?;
            on_added(rel)?;
        }
        out.into_inner()?.finish()?.flush()?;
        Ok(())
    })
}

/// Writes a kept entry again under its full `path`.
fn copy_entry<W: Write, R: Read>(
    out: &mut Builder<W>,
    entry: &mut tar::Entry<'_, R>,
    path: &Path,
) -> Result<()> {
    let mut header = entry.header().clone();
    match header.entry_type() {
        EntryType::Symlink | EntryType::Link => {
            let target = entry.link_name()?.map(Cow::into_owned).unwrap_or_default();
            out.append_link(&mut header, path, target)?;
        }
        EntryType::GNUSparse => {
            // The reader expands the holes: store the data as a plain file.
            header.set_entry_type(EntryType::Regular);
            header.set_size(entry.size());
            out.append_data(&mut header, path, entry)?;
        }
        _ => out.append_data(&mut header, path, entry)?,
    }
    Ok(())
}

/// A header for added data without a source file.
fn new_header(kind: EntryType, mode: u32, size: u64) -> Header {
    let mut header = Header::new_gnu();
    header.set_entry_type(kind);
    header.set_mode(mode);
    header.set_size(size);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    header.set_mtime(now);
    header
}

fn add_entry<W: Write>(out: &mut Builder<W>, rel: &Path, source: &EntrySource) -> Result<()> {
    let name = entry_name(rel, false);
    match source {
        EntrySource::Dir => {
            let mut header = new_header(EntryType::Directory, DIR_MODE, 0);
            out.append_data(&mut header, &name, io::empty())?;
        }
        EntrySource::File(path) => {
            let meta = fs::metadata(path)?;
            let mut header = Header::new_gnu();
            header.set_metadata_in_mode(&meta, HeaderMode::Complete);
            if meta.is_dir() {
                out.append_data(&mut header, &name, io::empty())?;
            } else {
                out.append_data(&mut header, &name, fs::File::open(path)?)?;
            }
        }
    }
    Ok(())
}
