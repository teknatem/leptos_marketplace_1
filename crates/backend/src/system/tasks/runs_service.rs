use anyhow::Result;
use contracts::system::tasks::aggregate::ScheduledTaskId;
use contracts::system::tasks::progress::TaskStatus;
use contracts::system::tasks::runs::TaskRun;
use std::collections::HashSet;

use super::{runs_repository, service as task_service};

/// Метрики выполнения задачи для записи в историю
pub struct RunMetrics {
    pub total_processed: Option<i64>,
    pub total_inserted: Option<i64>,
    pub total_updated: Option<i64>,
    pub total_errors: Option<i64>,
    pub http_request_count: Option<i64>,
    pub http_bytes_sent: Option<i64>,
    pub http_bytes_received: Option<i64>,
}

pub async fn reset_stale_running_runs(reason: &str) -> Result<u64> {
    let models = runs_repository::find_all_running_models()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))?;
    let n = models.len() as u64;
    let task_ids: Vec<String> = models
        .iter()
        .map(|m| m.task_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let reason_owned = reason.to_string();
    for m in models {
        finish_run(
            &m.session_id,
            TaskStatus::Failed,
            None,
            Some(reason_owned.clone()),
        )
        .await?;
    }
    task_service::reset_stale_task_statuses(&task_ids).await?;
    Ok(n)
}

pub async fn create_run(
    task_id: ScheduledTaskId,
    session_id: String,
    triggered_by: String,
) -> Result<()> {
    runs_repository::create_run(task_id, session_id, triggered_by, None)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn create_run_with_log(
    task_id: ScheduledTaskId,
    session_id: String,
    triggered_by: String,
    log_file_path: String,
) -> Result<()> {
    runs_repository::create_run(task_id, session_id, triggered_by, Some(log_file_path))
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn finish_run(
    session_id: &str,
    status: TaskStatus,
    metrics: Option<RunMetrics>,
    error_message: Option<String>,
) -> Result<()> {
    let (processed, inserted, updated, errors, http_n, http_up, http_down) = metrics
        .map(|m| {
            (
                m.total_processed,
                m.total_inserted,
                m.total_updated,
                m.total_errors,
                m.http_request_count,
                m.http_bytes_sent,
                m.http_bytes_received,
            )
        })
        .unwrap_or((None, None, None, None, None, None, None));

    runs_repository::finish_run(
        session_id,
        &status.to_string(),
        processed,
        inserted,
        updated,
        errors,
        http_n,
        http_up,
        http_down,
        error_message,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn get_run_by_session(session_id: &str) -> Result<Option<TaskRun>> {
    runs_repository::find_by_session_id(session_id)
        .await
        .map(|opt| {
            opt.map(|m| TaskRun {
                id: m.id,
                task_id: m.task_id,
                session_id: m.session_id,
                triggered_by: m.triggered_by,
                started_at: m.started_at,
                finished_at: m.finished_at,
                duration_ms: m.duration_ms,
                status: m.status,
                total_processed: m.total_processed,
                total_inserted: m.total_inserted,
                total_updated: m.total_updated,
                total_errors: m.total_errors,
                log_file_path: m.log_file_path,
                error_message: m.error_message,
                http_request_count: m.http_request_count,
                http_bytes_sent: m.http_bytes_sent,
                http_bytes_received: m.http_bytes_received,
                task_code: None,
                task_description: None,
            })
        })
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn get_running_for_task(task_id: &str) -> Result<Option<TaskRun>> {
    runs_repository::find_running_for_task_enriched(task_id)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_active() -> Result<Vec<TaskRun>> {
    runs_repository::list_active_enriched()
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_for_task(task_id: &str, limit: u64) -> Result<Vec<TaskRun>> {
    runs_repository::list_for_task(task_id, limit)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}

pub async fn list_recent(limit: u64) -> Result<Vec<TaskRun>> {
    runs_repository::list_recent(limit)
        .await
        .map_err(|e| anyhow::anyhow!("Database error: {}", e))
}
