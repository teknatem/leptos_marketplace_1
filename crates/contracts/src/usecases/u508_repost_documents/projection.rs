use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionOption {
    pub key: String,
    pub label: String,
    pub description: String,
}
