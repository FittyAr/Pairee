pub mod crc32;
pub mod md5;
pub mod sha1;
pub mod sha256;
pub mod blake3;

use super::options::HashAlgorithm;

pub trait HashStrategy: Send + Sync {
    /// Nombre legible del algoritmo ("BLAKE3", "SHA-256", etc.)
    fn name(&self) -> &str;
    /// Alimentar datos al hasher
    fn update(&mut self, data: &[u8]);
    /// Finalizar y producir el hash como string hexadecimal
    fn finalize(self: Box<Self>) -> String;
    /// Crear una nueva instancia limpia del mismo algoritmo
    fn new_instance(&self) -> Box<dyn HashStrategy>;
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
