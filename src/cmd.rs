use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Shared command between client and server, use for simpler communication.
pub enum CMD {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SetResponse {
    Ok(()),
    Err(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GetResponse {
    Ok(String),
    Err(String),
}

impl Display for GetResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetResponse::Ok(v) => f.write_str(v),
            GetResponse::Err(e) => f.write_str(e),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoveResponse {
    Ok(()),
    Err(String),
}
