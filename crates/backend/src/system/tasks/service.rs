use anyhow::Result;
use chrono::Utc;
use contracts::system::tasks::aggregate::{ScheduledTask, ScheduledTaskId};
use contracts::system::tasks::request::{CreateScheduledTaskDto, UpdateScheduledTaskDto};
use crate::system::tasks::repository;

pub async fn list_all() -> Result<Vec<ScheduledTask>> {
    repository::list_all().await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_enabled_tasks() -> Result<Vec<ScheduledTask>> {
    repository::list_enabled().await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn get_by_id(id: &ScheduledTaskId) -> Result<Option<ScheduledTask>> {
    repository::get_by_id(id.0).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn create(dto: CreateScheduledTaskDto) -> Result<ScheduledTaskId> {
    let task = ScheduledTask::new_for_insert(
        dto.code,
        dto.description,
        dto.task_type,
        dto.schedule_cron,
        dto.is_enabled,
        dto.config_json,
    );
    repository::save(&task).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    Ok(task.base.id)
}

pub async fn update(id: &ScheduledTaskId, dto: UpdateScheduledTaskDto) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;

    task.base.code = dto.code;
    task.base.description = dto.description;
    task.task_type = dto.task_type;
    task.schedule_cron = dto.schedule_cron;
    task.is_enabled = dto.is_enabled;
    task.config_json = dto.config_json;
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn delete(id: &ScheduledTaskId) -> Result<()> {
    repository::soft_delete(id.0).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn toggle_enabled(id: &ScheduledTaskId, is_enabled: bool) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;
    task.is_enabled = is_enabled;
    task.base.metadata.updated_at = Utc::now();
    repository::save(&task).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn update_run_status(
    id: &ScheduledTaskId,
    last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    last_run_log_file: Option<String>,
    last_run_status: Option<String>,
) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;
    
    task.last_run_at = last_run_at;
    task.next_run_at = next_run_at;
    task.last_run_log_file = last_run_log_file;
    task.last_run_status = last_run_status;
    task.base.metadata.updated_at = Utc::now();
    
    repository::save(&task).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
}
