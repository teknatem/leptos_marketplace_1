use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepostRequest {
    pub projection_key: String,
    pub date_from: String,
    pub date_to: String,
}
