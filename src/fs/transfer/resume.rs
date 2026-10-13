//! Resuming interrupted copies: a `.pairee-part` file left by an earlier
//! attempt (a failed retry, a crash, a power cut) is reused when its bytes
//! are exactly the beginning of the source; the copy then continues where
//! it stopped. Anything else is thrown away and the copy starts over.

use super::control::JobControl;
use super::hash::hash_reader;
use super::options::HashAlgorithm;
use std::io::{self, Read};
use std::path::Path;

/// Bytes of `part` that can be kept for a copy of `src` (0: start over; an
/// unusable `part` is deleted).
pub fn resume_offset(src: &Path, part: &Path, ctl: &JobControl) -> io::Result<u64> {
    let Ok(part_meta) = std::fs::symlink_metadata(part) else {
        return Ok(0);
    };
    let len = part_meta.len();
    let usable = part_meta.is_file()
        && len > 0
        && len <= std::fs::metadata(src)?.len()
        && same_prefix(src, part, len, ctl)?;
    if usable {
        return Ok(len);
    }
    super::part::discard(part);
    Ok(0)
}

/// `true` when the first `len` bytes of `a` and `b` hash the same.
fn same_prefix(a: &Path, b: &Path, len: u64, ctl: &JobControl) -> io::Result<bool> {
    Ok(prefix_hash(a, len, ctl)? == prefix_hash(b, len, ctl)?)
}

fn prefix_hash(path: &Path, len: u64, ctl: &JobControl) -> io::Result<String> {
    let mut prefix = std::fs::File::open(path)?.take(len);
    hash_reader(&mut prefix, HashAlgorithm::Blake3, &|| ctl.is_cancelled())
}
