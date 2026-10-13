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
    if old_size.max(new_size) > MAX_TEXT_BYTES {
        return Ok(FileComparison::Binary(BinaryComparison {
            identical: false,
            sizes: (old_size, new_size),
            hashes: (hash_file(old, cancelled)?, hash_file(new, cancelled)?),
            algorithm: HASH,
        })
        .settle());
    }
    let old_bytes = read_all(old)?;
    let new_bytes = read_all(new)?;
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

fn read_all(side: FileSide) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    side.vfs
        .open_read(side.path)?
        .take(MAX_TEXT_BYTES + 1)
        .read_to_end(&mut data)?;
    Ok(data)
}

/// `data` decoded as the viewer would (byte-order marks, UTF-16 and legacy
/// code pages detected); `None` for binary data.
fn as_text(data: &[u8]) -> Option<String> {
    decode_prefix(data, true, EncodingPolicy::Detect)
}
