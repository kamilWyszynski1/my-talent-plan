use std::ops::Deref;

use crate::{KvsEngine, Result};
use anyhow::Context;
use sled::Db;

/// Implements KvsEngine for sled database.
pub struct SledKvsEngine {
    db: Db,
}

impl SledKvsEngine {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(Self {
            db: sled::open(path)?,
        })
    }
}

impl KvsEngine for SledKvsEngine {
    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.db.insert(&key, value.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    fn get(&mut self, key: String) -> Result<Option<String>> {
        let value = self.db.get(key)?.context("Key not found")?;
        let a = value.deref();
        Ok(Some(String::from_utf8(a.to_vec())?))
    }

    fn remove(&mut self, key: String) -> Result<()> {
        self.db.remove(&key)?.context("Key not found")?;
        self.db.flush()?;
        Ok(())
    }
}
