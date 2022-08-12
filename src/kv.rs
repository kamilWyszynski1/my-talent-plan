use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct KvStore {
    data: HashMap<String, String>,
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::default(),
        }
    }

    pub fn set(&mut self, k: String, v: String) {
        self.data.insert(k, v);
    }

    pub fn get(&self, key: String) -> Option<String> {
        self.data.get(&key).map(|s| s.to_string())
    }

    pub fn remove(&mut self, key: String) {
        self.data.remove(&key);
    }
}
