use contracts::domain::a018_llm_chat::aggregate::LlmChatMessage;
use once_cell::sync::Lazy;
use sea_orm::{ConnectionTrait, DatabaseBackend, FromQueryResult, Statement};
use serde::Serialize;
use std::collections::HashSet;
use tokio::sync::RwLock;

const JOB_TTL_HOURS: i64 = 24;
static ACTIVE_JOBS: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));

#[derive(Debug, Clone, Serialize)]
pub struct JobProgress {
    pub step: u32,
    pub stage: String,
}

impl JobProgress {
    pub fn new(step: u32, stage: impl Into<String>) -> Self {
        Self {
            step,
            stage: stage.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LlmJobStatus {
    Pending(JobProgress),
    Done(LlmChatMessage),
    Error(String),
}

fn db() -> &'static sea_orm::DatabaseConnection {
    crate::shared::data::db::get_connection()
}

async fn cleanup_expired() {
    let _ = db()
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            "DELETE FROM a018_llm_job WHERE expires_at <= datetime('now')".to_string(),
        ))
        .await;
}

/// Registers a durable job. Repeating the same request_id for a chat returns
/// the original job instead of starting duplicate LLM work.
pub async fn register(
    job_id: &str,
    chat_id: &str,
    request_id: &str,
) -> anyhow::Result<(String, bool)> {
    cleanup_expired().await;
    let existing = serde_json::Value::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT id, status, error FROM a018_llm_job \
         WHERE chat_id = ? AND request_id = ? AND expires_at > datetime('now') LIMIT 1",
        vec![chat_id.into(), request_id.into()],
    ))
    .one(db())
    .await?;
    if let Some(id) = existing
        .as_ref()
        .and_then(|row| row.get("id"))
        .and_then(serde_json::Value::as_str)
    {
        let id = id.to_string();
        let status = existing
            .as_ref()
            .and_then(|row| row.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let interrupted = status == "error"
            && existing
                .as_ref()
                .and_then(|row| row.get("error"))
                .and_then(serde_json::Value::as_str)
                == Some("worker_interrupted");
        if interrupted {
            db().execute(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "UPDATE a018_llm_job SET status = 'pending', error = NULL, cancel_requested = 0, \
                 progress_step = 0, progress_stage = 'Повторный запуск…', updated_at = datetime('now') \
                 WHERE id = ?",
                vec![id.clone().into()],
            ))
            .await?;
        }
        let should_start =
            (status == "pending" || interrupted) && ACTIVE_JOBS.write().await.insert(id.clone());
        return Ok((id, should_start));
    }

    let result = db()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO a018_llm_job \
             (id, chat_id, request_id, status, progress_step, progress_stage, expires_at) \
             VALUES (?, ?, ?, 'pending', 0, 'Анализ запроса…', datetime('now', ?))",
            vec![
                job_id.into(),
                chat_id.into(),
                request_id.into(),
                format!("+{JOB_TTL_HOURS} hours").into(),
            ],
        ))
        .await;
    match result {
        Ok(_) => {
            ACTIVE_JOBS.write().await.insert(job_id.to_string());
            Ok((job_id.to_string(), true))
        }
        Err(error) => {
            // A concurrent retry may have won the unique(chat_id, request_id) race.
            let row = serde_json::Value::find_by_statement(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT id FROM a018_llm_job WHERE chat_id = ? AND request_id = ? LIMIT 1",
                vec![chat_id.into(), request_id.into()],
            ))
            .one(db())
            .await?;
            row.and_then(|value| {
                value
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(str::to_string)
            })
            .map(|id| (id, false))
            .ok_or_else(|| error.into())
        }
    }
}

pub async fn set_progress(job_id: &str, progress: JobProgress) {
    let _ = db()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE a018_llm_job SET progress_step = ?, progress_stage = ?, updated_at = datetime('now') \
             WHERE id = ? AND status = 'pending' AND cancel_requested = 0",
            vec![
                (progress.step as i64).into(),
                progress.stage.into(),
                job_id.into(),
            ],
        ))
        .await;
}

pub async fn complete(job_id: &str, msg: LlmChatMessage) {
    let result_json = serde_json::to_string(&msg).unwrap_or_else(|_| "null".to_string());
    let _ = db()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE a018_llm_job SET status = 'done', result_json = ?, updated_at = datetime('now') \
             WHERE id = ? AND status = 'pending' AND cancel_requested = 0",
            vec![result_json.into(), job_id.into()],
        ))
        .await;
    ACTIVE_JOBS.write().await.remove(job_id);
}

pub async fn fail(job_id: &str, error: String) {
    let _ = db()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE a018_llm_job SET status = 'error', error = ?, updated_at = datetime('now') \
             WHERE id = ? AND status = 'pending'",
            vec![error.into(), job_id.into()],
        ))
        .await;
    ACTIVE_JOBS.write().await.remove(job_id);
}

pub async fn cancel(job_id: &str) -> anyhow::Result<bool> {
    let result = db()
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "UPDATE a018_llm_job SET status = 'error', error = 'cancelled', cancel_requested = 1, \
             updated_at = datetime('now') WHERE id = ? AND status = 'pending'",
            vec![job_id.into()],
        ))
        .await?;
    ACTIVE_JOBS.write().await.remove(job_id);
    Ok(result.rows_affected() > 0)
}

pub async fn is_cancelled(job_id: &str) -> bool {
    serde_json::Value::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT cancel_requested FROM a018_llm_job WHERE id = ? LIMIT 1",
        vec![job_id.into()],
    ))
    .one(db())
    .await
    .ok()
    .flatten()
    .and_then(|row| {
        row.get("cancel_requested")
            .and_then(serde_json::Value::as_i64)
    })
    .unwrap_or(0)
        != 0
}

/// Reads status without deleting terminal state. TTL cleanup owns retention.
pub async fn take(job_id: &str) -> Option<LlmJobStatus> {
    cleanup_expired().await;
    let row = serde_json::Value::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT status, progress_step, progress_stage, result_json, error \
         FROM a018_llm_job WHERE id = ? AND expires_at > datetime('now') LIMIT 1",
        vec![job_id.into()],
    ))
    .one(db())
    .await
    .ok()
    .flatten()?;
    match row.get("status").and_then(serde_json::Value::as_str)? {
        "pending" if !ACTIVE_JOBS.read().await.contains(job_id) => {
            let _ = db()
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Sqlite,
                    "UPDATE a018_llm_job SET status = 'error', error = 'worker_interrupted', \
                     updated_at = datetime('now') WHERE id = ? AND status = 'pending'",
                    vec![job_id.into()],
                ))
                .await;
            Some(LlmJobStatus::Error(
                "worker_interrupted: повторите запрос с тем же request_id".to_string(),
            ))
        }
        "pending" => Some(LlmJobStatus::Pending(JobProgress::new(
            row.get("progress_step")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as u32,
            row.get("progress_stage")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
        ))),
        "done" => row
            .get("result_json")
            .and_then(serde_json::Value::as_str)
            .and_then(|json| serde_json::from_str(json).ok())
            .map(LlmJobStatus::Done),
        _ => Some(LlmJobStatus::Error(
            row.get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("LLM job failed")
                .to_string(),
        )),
    }
}
