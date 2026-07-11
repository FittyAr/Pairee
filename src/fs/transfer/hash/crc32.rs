use super::HashStrategy;
use crc32fast::Hasher;

pub struct Crc32Hasher {
    state: Hasher,
}

impl Crc32Hasher {
    pub fn new() -> Self {
        Self {
            state: Hasher::new(),
        }
    }
}

impl HashStrategy for Crc32Hasher {
    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let checksum = self.state.finalize();
        format!("{:08X}", checksum)
    }
}
