//! Resuming interrupted copies: a `.pairee-part` file left by an earlier
//! attempt (a failed retry, a crash, a power cut, a dropped connection) is
//! reused when its bytes are exactly the beginning of the source; the copy
//! then continues where it stopped. Anything else is thrown away and the
//! copy starts over. Both sides are read through their [`Vfs`] adapters, so
//! local and remote (SFTP) copies resume alike.

use super::control::JobControl;
use super::hash::hash_reader;
use super::options::HashAlgorithm;
use crate::fs::vfs::Vfs;
use std::io::{self, Read};
use std::path::Path;

/// One side of the check: a file on an adapter.
pub struct Side<'a> {
    pub vfs: &'a dyn Vfs,
    pub path: &'a Path,
}

/// Bytes of `part` that can be kept for a copy of `src` (0: start over; an
/// unusable `part` is deleted).
pub fn resume_offset(src: Side, part: Side, ctl: &JobControl) -> io::Result<u64> {
    let Ok(found) = part.vfs.stat(part.path) else {
        return Ok(0);
    };
    let len = found.size;
    let usable = !found.is_dir
        && !found.is_symlink
        && len > 0
        && len <= src.vfs.metadata(src.path)?.size
        && prefix_hash(&src, len, ctl)? == prefix_hash(&part, len, ctl)?;
    if usable {
        return Ok(len);
    }
    let _ = part.vfs.remove_file(part.path);
    Ok(0)
}

fn prefix_hash(side: &Side, len: u64, ctl: &JobControl) -> io::Result<String> {
    let mut prefix = side.vfs.open_read(side.path)?.take(len);
    hash_reader(&mut prefix, HashAlgorithm::Blake3, &|| ctl.is_cancelled())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::transfer::job::{TransferJob, TransferOperation};
    use crate::fs::transfer::options::TransferOptions;
    use crate::fs::vfs::LocalVfs;
    use std::path::PathBuf;

    fn control() -> JobControl {
        let (tx, _rx) = crate::fs::transfer::events::EventSender::channel();
        let job = TransferJob::new(
            TransferOperation::Copy,
            vec![],
            PathBuf::new(),
            TransferOptions::default(),
        );
        JobControl::for_job(&job, tx)
    }

    fn offset(src: &Path, part: &Path) -> u64 {
        let side = |path| Side {
            vfs: &LocalVfs,
            path,
        };
        resume_offset(side(src), side(part), &control()).unwrap()
    }

    #[test]
    fn keeps_a_matching_prefix_and_drops_anything_else() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let part = dir.path().join("dst.bin.pairee-part");
        std::fs::write(&src, b"0123456789").unwrap();
        assert_eq!(offset(&src, &part), 0);
        std::fs::write(&part, b"0123").unwrap();
        assert_eq!(offset(&src, &part), 4);
        std::fs::write(&part, b"0X23").unwrap();
        assert_eq!(offset(&src, &part), 0);
        assert!(!part.exists());
        std::fs::write(&part, b"0123456789-longer").unwrap();
        assert_eq!(offset(&src, &part), 0);
        assert!(!part.exists());
    }
}
