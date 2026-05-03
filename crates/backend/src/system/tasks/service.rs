use crate::system::tasks::repository;
use anyhow::Result;
use chrono::Utc;
use contracts::system::tasks::aggregate::{ScheduledTask, ScheduledTaskId};
use contracts::system::tasks::request::{CreateScheduledTaskDto, UpdateScheduledTaskDto};

pub async fn list_all() -> Result<Vec<ScheduledTask>> {
    repository::list_all()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_enabled_tasks() -> Result<Vec<ScheduledTask>> {
    repository::list_enabled()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn get_by_id(id: &ScheduledTaskId) -> Result<Option<ScheduledTask>> {
    repository::get_by_id(id.0)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn create(dto: CreateScheduledTaskDto) -> Result<ScheduledTaskId> {
    let task = ScheduledTask::new_for_insert(
        dto.code,
        dto.description,
        dto.comment,
        dto.task_type,
        dto.schedule_cron,
        dto.is_enabled,
        dto.config_json,
    );
    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    Ok(task.base.id)
}

pub async fn update(id: &ScheduledTaskId, dto: UpdateScheduledTaskDto) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;

    task.base.code = dto.code;
    task.base.description = dto.description;
    task.base.comment = dto.comment;
    task.task_type = dto.task_type;
    task.schedule_cron = dto.schedule_cron;
    task.is_enabled = dto.is_enabled;
    task.config_json = dto.config_json;
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn delete(id: &ScheduledTaskId) -> Result<()> {
    repository::soft_delete(id.0)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn toggle_enabled(id: &ScheduledTaskId, is_enabled: bool) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;
    task.is_enabled = is_enabled;
    task.base.metadata.updated_at = Utc::now();
    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn update_run_status(
    id: &ScheduledTaskId,
    last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    last_run_log_file: Option<String>,
    last_run_status: Option<String>,
    last_successful_run_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;

    task.last_run_at = last_run_at;
    task.next_run_at = next_run_at;
    task.last_run_log_file = last_run_log_file;
    task.last_run_status = last_run_status;
    // Only overwrite watermark when a successful timestamp is provided.
    if let Some(ts) = last_successful_run_at {
        task.last_successful_run_at = Some(ts);
    }
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

/// Устанавливает или сбрасывает watermark (last_successful_run_at) задачи.
/// `date_str` = "YYYY-MM-DD" → watermark = указанная дата 23:59:59 UTC.
/// `None` → watermark сбрасывается в NULL, следующий запуск начнёт с work_start_date.
/// Синхронизирует `last_run_status` в `sys_tasks` после сброса зомби-запусков.
///
/// Вызывается только из `runs_service::reset_stale_running_runs` при старте сервера.
pub async fn reset_stale_task_statuses(task_ids: &[String]) -> Result<()> {
    repository::set_failed_if_status_running(task_ids)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn set_watermark(id: &ScheduledTaskId, date_str: Option<&str>) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;

    task.last_successful_run_at = match date_str {
        None => None,
        Some(s) => {
            let date = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                anyhow::anyhow!("Invalid date format: expected YYYY-MM-DD, got {s}")
            })?;
            Some(
                date.and_hms_opt(23, 59, 59)
                    .map(|t| t.and_utc())
                    .ok_or_else(|| anyhow::anyhow!("Date overflow"))?,
            )
        }
    };
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}
