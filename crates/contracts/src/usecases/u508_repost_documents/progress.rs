use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepostProgress {
    pub session_id: String,
    pub status: RepostStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub processed: i32,
    pub total: Option<i32>,
    pub reposted: i32,
    pub errors: i32,
    pub current_item: Option<String>,
    pub error_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RepostStatus {
    Running,
    Completed,
    CompletedWithErrors,
    Failed,
}

impl RepostProgress {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            status: RepostStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
            updated_at: Utc::now(),
            processed: 0,
            total: None,
            reposted: 0,
            errors: 0,
            current_item: None,
            error_messages: Vec::new(),
        }
    }
}
