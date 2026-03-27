use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateOption {
    pub key: String,
    pub label: String,
    pub description: String,
}
