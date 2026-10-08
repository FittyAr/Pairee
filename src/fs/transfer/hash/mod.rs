pub mod blake3;
pub mod crc32;
pub mod md5;
pub mod sha1;
pub mod sha256;
#[cfg(test)]
mod tests;

use super::options::HashAlgorithm;

pub trait HashStrategy: Send + Sync {
    /// Alimentar datos al hasher
    fn update(&mut self, data: &[u8]);
    /// Finalizar y producir el hash como string hexadecimal
    fn finalize(self: Box<Self>) -> String;
}

pub fn create_hasher(algorithm: HashAlgorithm) -> Box<dyn HashStrategy> {
    match algorithm {
        HashAlgorithm::Crc32 => Box::new(crc32::Crc32Hasher::new()),
        HashAlgorithm::Md5 => Box::new(md5::Md5Hasher::new()),
        HashAlgorithm::Sha1 => Box::new(sha1::Sha1Hasher::new()),
        HashAlgorithm::Sha256 => Box::new(sha256::Sha256Hasher::new()),
        HashAlgorithm::Blake3 => Box::new(blake3::Blake3Hasher::new()),
    }
}

/// Block size for [`hash_reader`].
const HASH_BLOCK: usize = 256 * 1024;

/// Hashes everything `file` yields. `cancelled` is polled between blocks;
/// a cancelled run fails with [`std::io::ErrorKind::Interrupted`].
pub fn hash_reader(
    file: &mut dyn std::io::Read,
    algorithm: HashAlgorithm,
    cancelled: &dyn Fn() -> bool,
) -> std::io::Result<String> {
    let mut hasher = create_hasher(algorithm);
    let mut buf = vec![0u8; HASH_BLOCK];
    loop {
        if cancelled() {
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(hasher.finalize());
        }
        hasher.update(&buf[..n]);
    }
}
