use crate::system::tasks::change_token;
use crate::system::tasks::repository;
use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use contracts::system::tasks::aggregate::{ScheduledTask, ScheduledTaskId};
use contracts::system::tasks::request::{CreateScheduledTaskDto, UpdateScheduledTaskDto};
use std::str::FromStr;

fn next_run_from_cron(schedule_cron: &str, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let schedule = cron::Schedule::from_str(schedule_cron).ok()?;
    schedule.after(&after).next()
}

fn next_run_for_enabled_schedule(
    schedule_cron: &Option<String>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    schedule_cron
        .as_deref()
        .and_then(|cron| next_run_from_cron(cron, now))
}

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
    let mut task = ScheduledTask::new_for_insert(
        dto.code,
        dto.description,
        dto.comment,
        dto.task_type,
        dto.schedule_cron,
        dto.is_enabled,
        dto.config_json,
    );
    if task.is_enabled {
        task.next_run_at = next_run_for_enabled_schedule(&task.schedule_cron, Utc::now());
    }
    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    change_token::TOKEN.bump();
    Ok(task.base.id)
}

pub async fn update(id: &ScheduledTaskId, dto: UpdateScheduledTaskDto) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;
    let was_enabled = task.is_enabled;
    let old_schedule_cron = task.schedule_cron.clone();

    task.base.code = dto.code;
    task.base.description = dto.description;
    task.base.comment = dto.comment;
    task.task_type = dto.task_type;
    task.schedule_cron = dto.schedule_cron;
    task.is_enabled = dto.is_enabled;
    task.config_json = dto.config_json;
    if task.is_enabled && (!was_enabled || task.schedule_cron != old_schedule_cron) {
        task.next_run_at = next_run_for_enabled_schedule(&task.schedule_cron, Utc::now());
    }
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    change_token::TOKEN.bump();
    Ok(())
}

pub async fn delete(id: &ScheduledTaskId) -> Result<()> {
    repository::soft_delete(id.0)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    change_token::TOKEN.bump();
    Ok(())
}

pub async fn toggle_enabled(id: &ScheduledTaskId, is_enabled: bool) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;
    task.is_enabled = is_enabled;
    if is_enabled {
        task.next_run_at = next_run_for_enabled_schedule(&task.schedule_cron, Utc::now());
    }
    task.base.metadata.updated_at = Utc::now();
    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    change_token::TOKEN.bump();
    Ok(())
}

pub async fn update_run_status(
    id: &ScheduledTaskId,
    last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    last_run_log_file: Option<String>,
    last_run_status: Option<String>,
    last_successful_run_at: Option<chrono::DateTime<chrono::Utc>>,
    data_loaded_up_to: Option<NaiveDate>,
) -> Result<()> {
    let mut task = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Scheduled task not found"))?;

    task.last_run_at = last_run_at;
    task.next_run_at = next_run_at;
    task.last_run_log_file = last_run_log_file;
    task.last_run_status = last_run_status;
    // Only overwrite success fields when a successful run provides them.
    if let Some(ts) = last_successful_run_at {
        task.last_successful_run_at = Some(ts);
    }
    if let Some(date) = data_loaded_up_to {
        task.data_loaded_up_to = Some(date);
    }
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    change_token::TOKEN.bump();
    Ok(())
}

/// Устанавливает или сбрасывает date-only watermark данных (`data_loaded_up_to`) задачи.
/// `date_str` = "YYYY-MM-DD" → данные считаются загруженными включительно по указанную дату.
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

    task.data_loaded_up_to = match date_str {
        None => {
            task.last_successful_run_at = None;
            None
        }
        Some(s) => {
            let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                anyhow::anyhow!("Invalid date format: expected YYYY-MM-DD, got {s}")
            })?;
            Some(date)
        }
    };
    task.base.metadata.updated_at = Utc::now();

    repository::save(&task)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}
