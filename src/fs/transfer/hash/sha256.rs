use super::HashStrategy;
use sha2::{Digest, Sha256};

pub struct Sha256Hasher {
    state: Sha256,
}

impl Sha256Hasher {
    pub fn new() -> Self {
        Self {
            state: Sha256::new(),
        }
    }
}

impl HashStrategy for Sha256Hasher {
    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.state.finalize();
        format!("{:064X}", result)
    }
}
