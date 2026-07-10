use super::HashStrategy;
use sha1::{Sha1, Digest};

pub struct Sha1Hasher {
    state: Sha1,
}

impl Sha1Hasher {
    pub fn new() -> Self {
        Self {
            state: Sha1::new(),
        }
    }
}

impl HashStrategy for Sha1Hasher {
    fn name(&self) -> &str {
        "SHA-1"
    }

    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let result = self.state.finalize();
        format!("{:040X}", result)
    }

    fn new_instance(&self) -> Box<dyn HashStrategy> {
        Box::new(Sha1Hasher::new())
    }
}
