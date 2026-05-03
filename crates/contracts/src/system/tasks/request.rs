use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduledTaskDto {
    pub code: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub task_type: String,
    pub schedule_cron: Option<String>,
    pub is_enabled: bool,
    pub config_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduledTaskDto {
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub task_type: String,
    pub schedule_cron: Option<String>,
    pub is_enabled: bool,
    pub config_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleScheduledTaskEnabledDto {
    pub is_enabled: bool,
}

/// Установить или сбросить watermark (last_successful_run_at) задачи.
/// `date` — дата в формате "YYYY-MM-DD", null — полный сброс (будет загружено с work_start_date).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetWatermarkDto {
    /// "YYYY-MM-DD" или null для полного сброса
    pub date: Option<String>,
}
