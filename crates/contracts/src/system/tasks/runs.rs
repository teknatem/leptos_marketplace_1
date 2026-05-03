use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::progress::TaskProgressResponse;

/// Запись о конкретном запуске регламентного задания
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: String,
    pub task_id: String,
    pub task_code: Option<String>,
    pub task_description: Option<String>,
    pub session_id: String,
    pub triggered_by: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub total_processed: Option<i64>,
    pub total_inserted: Option<i64>,
    pub total_updated: Option<i64>,
    pub total_errors: Option<i64>,
    pub log_file_path: Option<String>,
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_request_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_bytes_sent: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_bytes_received: Option<i64>,
}

/// Ответ со списком запусков конкретной задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRunListResponse {
    pub task_id: String,
    pub runs: Vec<TaskRun>,
}

/// Ответ с последними запусками всех задач (для мониторинга)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRunsResponse {
    pub runs: Vec<TaskRun>,
}

/// Ответ на ручной запуск задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTaskResponse {
    pub session_id: String,
    pub task_id: String,
}

/// Сессия прогресса только из памяти (без `sys_task_runs`, без чтения логов с диска).
/// Нужна лёгкая панель мониторинга, не конкурирующая с воркером за БД.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMemoryProgressItem {
    pub task_type: String,
    pub task_display_name: String,
    pub progress: TaskProgressResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveMemoryProgressResponse {
    pub items: Vec<LiveMemoryProgressItem>,
}
