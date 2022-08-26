use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Shared command between client and server, use for simpler communication.
pub enum CMD {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}
