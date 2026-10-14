//! Compare files: a line diff of two text files, or for binary (and very
//! large) files whether they are identical, with their hashes. Both files
//! are read through their [`Vfs`], so local, SFTP and archive files mix.

use super::DiffDocument;
use crate::fs::text::encoding::{EncodingPolicy, decode_prefix};
use crate::fs::transfer::hash::hash_reader;
use crate::fs::transfer::options::HashAlgorithm;
use crate::fs::vfs::Vfs;
use std::io::{self, Read};
use std::path::Path;

/// Larger files are compared by hash only.
pub const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;
/// Hash shown for binary files.
pub const HASH: HashAlgorithm = HashAlgorithm::Blake3;

/// One of the two files.
#[derive(Clone, Copy)]
pub struct FileSide<'a> {
    pub vfs: &'a dyn Vfs,
    pub path: &'a Path,
}

/// Binary files: sizes and hashes of both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryComparison {
    pub identical: bool,
    pub sizes: (u64, u64),
    pub hashes: (String, String),
    pub algorithm: HashAlgorithm,
}

/// What comparing two files found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileComparison {
    /// Text files: the hunks (none when equal).
    Text(DiffDocument),
    Binary(BinaryComparison),
}

/// Compares `old` with `new`. `cancelled` stops hashing large files.
pub fn compare_files(
    old: FileSide,
    new: FileSide,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<FileComparison> {
    let old_size = old.vfs.metadata(old.path)?.size;
    let new_size = new.vfs.metadata(new.path)?.size;
    let by_hash = || -> io::Result<FileComparison> {
        Ok(FileComparison::Binary(BinaryComparison {
            identical: false,
            sizes: (old_size, new_size),
            hashes: (hash_file(old, cancelled)?, hash_file(new, cancelled)?),
            algorithm: HASH,
        })
        .settle())
    };
    if old_size.max(new_size) > MAX_TEXT_BYTES {
        return by_hash();
    }
    // A file that grew past the limit since its size was read is compared
    // whole by hash, never diffed truncated.
    let (Some(old_bytes), Some(new_bytes)) = (read_all(old)?, read_all(new)?) else {
        return by_hash();
    };
    match (as_text(&old_bytes), as_text(&new_bytes)) {
        (Some(a), Some(b)) => Ok(FileComparison::Text(super::text::diff_texts(&a, &b))),
        _ => Ok(FileComparison::Binary(BinaryComparison {
            identical: false,
            sizes: (old_bytes.len() as u64, new_bytes.len() as u64),
            hashes: (
                hash_reader(&mut old_bytes.as_slice(), HASH, cancelled)?,
                hash_reader(&mut new_bytes.as_slice(), HASH, cancelled)?,
            ),
            algorithm: HASH,
        })
        .settle()),
    }
}

impl FileComparison {
    /// Sets `identical` of a binary comparison from its sizes and hashes.
    fn settle(self) -> Self {
        match self {
            Self::Binary(mut b) => {
                b.identical = b.sizes.0 == b.sizes.1 && b.hashes.0 == b.hashes.1;
                Self::Binary(b)
            }
            text => text,
        }
    }
}

fn hash_file(side: FileSide, cancelled: &dyn Fn() -> bool) -> io::Result<String> {
    let mut reader = side.vfs.open_read(side.path)?;
    hash_reader(&mut reader, HASH, cancelled)
}

/// The whole file, or `None` when it is larger than [`MAX_TEXT_BYTES`].
fn read_all(side: FileSide) -> io::Result<Option<Vec<u8>>> {
    let mut data = Vec::new();
    side.vfs
        .open_read(side.path)?
        .take(MAX_TEXT_BYTES + 1)
        .read_to_end(&mut data)?;
    Ok((data.len() as u64 <= MAX_TEXT_BYTES).then_some(data))
}

/// `data` decoded as the viewer would (byte-order marks, UTF-16 and legacy
/// code pages detected); `None` for binary data.
fn as_text(data: &[u8]) -> Option<String> {
    decode_prefix(data, true, EncodingPolicy::Detect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_past_the_limit_is_not_read_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        std::fs::write(&path, vec![b'a'; MAX_TEXT_BYTES as usize + 1]).unwrap();
        let side = FileSide {
            vfs: &crate::fs::vfs::LocalVfs,
            path: &path,
        };
        assert!(read_all(side).unwrap().is_none());
        std::fs::write(&path, b"small").unwrap();
        assert_eq!(read_all(side).unwrap().as_deref(), Some(&b"small"[..]));
    }
}
