use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduledTaskDto {
    pub code: String,
    pub description: String,
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


