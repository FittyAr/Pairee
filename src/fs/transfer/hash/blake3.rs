use super::HashStrategy;
use blake3::Hasher;

pub struct Blake3Hasher {
    state: Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self {
            state: Hasher::new(),
        }
    }
}

impl HashStrategy for Blake3Hasher {
    fn name(&self) -> &str {
        "BLAKE3"
    }

    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    fn finalize(self: Box<Self>) -> String {
        let hash = self.state.finalize();
        hash.to_hex().to_string().to_uppercase()
    }

    fn new_instance(&self) -> Box<dyn HashStrategy> {
        Box::new(Blake3Hasher::new())
    }
}
