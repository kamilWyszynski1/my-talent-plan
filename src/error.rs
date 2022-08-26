use thiserror::Error;

#[derive(Error, Debug)]
pub enum KvsError {
    #[error("Key not found")]
    /// Key was not found during removal.
    KeyNotFound,
    #[error("Could not parse")]
    Parse,
}

pub type Result<T> = std::result::Result<T, anyhow::Error>;
