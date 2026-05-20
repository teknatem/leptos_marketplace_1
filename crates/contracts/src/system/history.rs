use serde::{Deserialize, Serialize};

/// One recorded page-open event for a user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageHistoryDto {
    pub id: String,
    pub owner_user_id: String,
    pub tab_key: String,
    pub title: String,
    pub opened_at: String,
}

/// Request to record a page open. Sent centrally from `open_tab`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageHistoryRecordRequest {
    pub tab_key: String,
    pub title: String,
}

/// Optional query for listing history (limit number of recent rows).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PageHistoryListQuery {
    pub limit: Option<u64>,
}

/// Default number of rows returned when no limit is provided.
pub const PAGE_HISTORY_DEFAULT_LIMIT: u64 = 100;

/// Maximum number of rows kept per user (older rows are pruned on record).
pub const PAGE_HISTORY_MAX_ROWS: u64 = 300;
