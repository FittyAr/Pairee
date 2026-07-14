use super::HashStrategy;
use md5::{Digest, Md5};

pub struct Md5Hasher {
    state: Md5,
}

impl Md5Hasher {
    pub fn new() -> Self {
        Self { state: Md5::new() }
    }
}

impl HashStrategy for Md5Hasher {
    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.state.finalize();
        format!("{:032X}", result)
    }
}
