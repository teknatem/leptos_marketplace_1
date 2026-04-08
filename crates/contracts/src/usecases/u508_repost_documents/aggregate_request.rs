use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateRepostRequest {
    pub aggregate_key: String,
    pub date_from: String,
    pub date_to: String,
    pub only_posted: bool,
    #[serde(default)]
    pub connection_mp_refs: Vec<String>,
}
