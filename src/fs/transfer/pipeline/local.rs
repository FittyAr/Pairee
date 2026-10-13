//! The local ends of the byte pipeline: a Direct I/O reader of the source
//! and a writer of the `.pairee-part` file that uses Direct I/O for whole
//! aligned blocks and can leave holes (sparse files).

use super::Sink;
use crate::fs::transfer::direct_io::{
    ALIGN, AlignedBuffer, DiskReader, open_writer_direct, to_long_path,
};
use crate::fs::transfer::options::TransferOptions;
use crate::fs::transfer::sparse;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// The local file `src` read from byte `start` (Direct I/O when asked), and
/// its size.
pub fn local_source(
    src: &Path,
    start: u64,
    options: &TransferOptions,
) -> io::Result<(Box<dyn Read + Send>, u64)> {
    let src = to_long_path(src);
    let size = std::fs::metadata(&src)?.len();
    let block = options.buffer_size.to_bytes();
    let reader = DiskReader::open(&src, options.direct_io, start, block)?;
    Ok((Box::new(reader), size))
}

/// Writer of a local `.pairee-part` file.
pub struct LocalSink {
    file: File,
    /// Direct I/O handle, used while blocks stay whole and aligned.
    direct: Option<File>,
    aligned: Option<AlignedBuffer>,
    sparse: bool,
    written: u64,
}

impl LocalSink {
    /// Opens `part` for a copy that already has its first `offset` bytes
    /// (0: created or truncated); missing parent folders are created.
    pub fn open(part: &Path, offset: u64, options: &TransferOptions) -> io::Result<Self> {
        let part = to_long_path(part);
        if let Some(parent) = part.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = if offset > 0 {
            OpenOptions::new().write(true).open(&part)
        } else {
            File::create(&part)
        }?;
        if offset > 0 {
            file.set_len(offset)?;
            file.seek(SeekFrom::Start(offset))?;
        }
        // Holes need the sparse flag (Windows); without it zeros are written.
        let sparse = options.produce_sparse_files && sparse::mark_sparse(&file).is_ok();
        // Direct I/O opens (and truncates) the file again: only for a fresh,
        // non-sparse copy.
        let direct = (options.direct_io && offset == 0 && !sparse)
            .then(|| open_writer_direct(&part, true).ok())
            .flatten();
        let aligned = direct
            .is_some()
            .then(|| AlignedBuffer::new(options.buffer_size.to_bytes(), ALIGN));
        Ok(Self {
            file,
            direct,
            aligned,
            sparse,
            written: offset,
        })
    }
}

impl Sink for LocalSink {
    fn write_block(&mut self, chunk: &[u8]) -> io::Result<()> {
        let len = chunk.len();
        if let (Some(f), Some(buf)) = (self.direct.as_mut(), self.aligned.as_mut())
            && len >= ALIGN
            && len.is_multiple_of(ALIGN)
        {
            buf.as_mut_slice()[..len].copy_from_slice(chunk);
            if f.write_all(&buf.as_slice()[..len]).is_ok() {
                self.written += len as u64;
                return Ok(());
            }
        }
        if self.direct.take().is_some() {
            // Unaligned tail or Direct I/O error: continue buffered.
            self.file.seek(SeekFrom::Start(self.written))?;
        }
        if self.sparse {
            sparse::write_sparse(&mut self.file, chunk)?;
        } else {
            self.file.write_all(chunk)?;
        }
        self.written += len as u64;
        Ok(())
    }

    fn finish(self: Box<Self>) -> io::Result<()> {
        if self.sparse {
            // A trailing hole is only a seek: give the file its length.
            self.file.set_len(self.written)?;
        }
        match self.direct {
            Some(f) => f.sync_all(),
            None => self.file.sync_all(),
        }
    }
}
