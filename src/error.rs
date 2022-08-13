use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvsError {
    #[error("Key not found")]
    /// Key was not found during removal.
    KeyNotFound,
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;
