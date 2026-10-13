//! "Verify after copy": the written file is read back from the disk and
//! hashed, so the check covers what actually landed on the storage (not the
//! bytes that were in memory while copying).

use super::direct_io::DiskReader;
use super::hash::hash_reader;
use super::options::HashAlgorithm;
use std::io;
use std::path::Path;

/// Block read while hashing.
const BLOCK: usize = 1024 * 1024;

/// Hash of the file at `path`, read from the disk (bypassing the cache with
/// `direct` where the platform allows). `cancelled` is polled per block.
pub fn hash_file_on_disk(
    path: &Path,
    algorithm: HashAlgorithm,
    direct: bool,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<String> {
    hash_reader(
        &mut DiskReader::open(path, direct, 0, BLOCK)?,
        algorithm,
        cancelled,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_match_for_equal_files_and_differ_otherwise() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        let data: Vec<u8> = (0..(BLOCK + 123)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&a, &data).unwrap();
        std::fs::write(&b, &data).unwrap();
        let hash = |p: &Path, direct| {
            hash_file_on_disk(p, HashAlgorithm::Blake3, direct, &|| false).unwrap()
        };
        assert_eq!(hash(&a, false), hash(&b, true));
        std::fs::write(&b, &data[1..]).unwrap();
        assert_ne!(hash(&a, false), hash(&b, false));
        let cancelled = hash_file_on_disk(&a, HashAlgorithm::Crc32, false, &|| true);
        assert_eq!(cancelled.unwrap_err().kind(), io::ErrorKind::Interrupted);
    }
}
