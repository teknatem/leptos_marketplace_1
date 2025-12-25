use serde::{Deserialize, Serialize};
use crate::system::sys_scheduled_task::aggregate::ScheduledTask;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskResponse {
    pub id: String,
    pub code: String,
    pub description: String,
    pub comment: Option<String>,
    pub task_type: String,
    pub schedule_cron: Option<String>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_enabled: bool,
    pub config_json: String,
    pub last_run_log_file: Option<String>,
    pub last_run_status: Option<String>,
}

impl From<ScheduledTask> for ScheduledTaskResponse {
    fn from(task: ScheduledTask) -> Self {
        use crate::domain::common::AggregateId;
        Self {
            id: task.base.id.as_string(),
            code: task.base.code,
            description: task.base.description,
            comment: task.base.comment,
            task_type: task.task_type,
            schedule_cron: task.schedule_cron,
            last_run_at: task.last_run_at,
            next_run_at: task.next_run_at,
            is_enabled: task.is_enabled,
            config_json: task.config_json,
            last_run_log_file: task.last_run_log_file,
            last_run_status: task.last_run_status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTaskListResponse {
    pub tasks: Vec<ScheduledTaskResponse>,
}


