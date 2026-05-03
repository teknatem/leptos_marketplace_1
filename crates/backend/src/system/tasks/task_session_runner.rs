//! Единственное место, где живёт полный жизненный цикл одного запуска задачи:
//! `timeout` → `QUOTA_EXHAUSTED` → watermark → `RunMetrics` → `finish_run`.
//!
//! Используется как воркером (плановый запуск), так и HTTP-хендлером (ручной запуск).
//! Любое изменение политики завершения правится ровно здесь.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use contracts::domain::common::AggregateId;
use contracts::system::tasks::aggregate::{ScheduledTask, ScheduledTaskId};
use contracts::system::tasks::progress::TaskStatus;

use super::{
    abort_registry, logger::TaskLogger, registry::TaskManagerRegistry, runs_service, service,
};

fn progress_to_run_metrics(
    progress: contracts::system::tasks::progress::TaskProgress,
) -> runs_service::RunMetrics {
    runs_service::RunMetrics {
        total_processed: progress.processed_items.map(|x| x as i64),
        total_inserted: progress.total_inserted.map(|x| x as i64),
        total_updated: progress.total_updated.map(|x| x as i64),
        total_errors: progress.errors.as_ref().map(|e| e.len() as i64),
        http_request_count: progress.http_request_count.map(|x| x as i64),
        http_bytes_sent: progress.http_bytes_sent,
        http_bytes_received: progress.http_bytes_received,
    }
}

/// Параметры одного выполнения регламентного задания.
pub struct TaskSessionParams {
    /// Полная запись задачи (нужна менеджеру для чтения конфига).
    pub task: ScheduledTask,
    /// UUID сессии (уже записан в `sys_task_runs` вызывающей стороной).
    pub session_id: String,
    /// Момент начала запуска (UTC); используется для watermark и `update_run_status`.
    pub started_at: DateTime<Utc>,
    /// Следующий плановый запуск:
    /// - воркер вычисляет из cron-выражения;
    /// - ручной запуск сохраняет текущий `task.next_run_at`.
    pub next_run_at: DateTime<Utc>,
    pub logger: Arc<TaskLogger>,
    pub registry: Arc<TaskManagerRegistry>,
}

/// Запускает задачу в фоновом Tokio-задании и регистрирует `AbortHandle` в реестре.
///
/// Функция возвращается немедленно; вся post-run логика выполняется асинхронно
/// внутри порождённого задания.
pub fn spawn_task_session(params: TaskSessionParams) {
    let TaskSessionParams {
        task,
        session_id,
        started_at,
        next_run_at,
        logger,
        registry,
    } = params;

    let task_id: ScheduledTaskId = task.base.id;
    let task_type = task.task_type.clone();
    let task_description = task.base.description.clone();
    let session_id_clone = session_id.clone();

    let timeout_secs = registry
        .get(&task_type)
        .map(|m| m.metadata().max_duration_seconds)
        .unwrap_or(7200);

    let join_handle = tokio::spawn(async move {
        let manager = match registry.get(&task_type) {
            Some(m) => m,
            None => {
                tracing::warn!(
                    "spawn_task_session: no manager for type '{}' (task '{}' {})",
                    task_type,
                    task_description,
                    task_id.as_string()
                );
                let _ = service::update_run_status(
                    &task_id,
                    Some(started_at),
                    Some(next_run_at),
                    None,
                    Some(TaskStatus::Failed.to_string()),
                    None,
                )
                .await;
                let _ = runs_service::finish_run(
                    &session_id_clone,
                    TaskStatus::Failed,
                    None,
                    Some(format!("No manager found for task type '{task_type}'")),
                )
                .await;
                abort_registry::remove(&session_id_clone);
                return;
            }
        };

        let run_fut = manager.run(&task, &session_id_clone, logger);
        let timed = tokio::time::timeout(Duration::from_secs(timeout_secs), run_fut).await;

        match timed {
            Ok(Ok(outcome)) => {
                let lsra_opt = if outcome.advances_watermark() {
                    let today = started_at.date_naive();
                    Some(match outcome.loaded_to {
                        Some(loaded_to) if loaded_to < today => loaded_to
                            .and_hms_opt(23, 59, 59)
                            .map(|t| t.and_utc())
                            .unwrap_or(started_at),
                        _ => started_at,
                    })
                } else {
                    None
                };

                tracing::info!(
                    "Task '{}' ({}) session {} finished: {}",
                    task_description,
                    task_id.as_string(),
                    session_id_clone,
                    outcome.status.to_string()
                );

                let metrics = manager
                    .get_progress(&session_id_clone)
                    .map(progress_to_run_metrics);

                let _ = service::update_run_status(
                    &task_id,
                    Some(started_at),
                    Some(next_run_at),
                    None,
                    Some(outcome.status.to_string()),
                    lsra_opt,
                )
                .await;
                let _ = runs_service::finish_run(&session_id_clone, outcome.status, metrics, None)
                    .await;
            }

            Ok(Err(e)) => {
                let error_str = e.to_string();
                let next_run_actual = if error_str.starts_with("QUOTA_EXHAUSTED:") {
                    tracing::warn!(
                        "Task '{}' ({}) quota exhausted; next run postponed 24h",
                        task_description,
                        task_id.as_string()
                    );
                    started_at + chrono::Duration::hours(24)
                } else {
                    next_run_at
                };

                tracing::error!(
                    "Task '{}' ({}) session {} failed: {}",
                    task_description,
                    task_id.as_string(),
                    session_id_clone,
                    error_str
                );
                let _ = service::update_run_status(
                    &task_id,
                    Some(started_at),
                    Some(next_run_actual),
                    None,
                    Some(TaskStatus::Failed.to_string()),
                    None,
                )
                .await;
                let _ = runs_service::finish_run(
                    &session_id_clone,
                    TaskStatus::Failed,
                    manager
                        .get_progress(&session_id_clone)
                        .map(progress_to_run_metrics),
                    Some(error_str),
                )
                .await;
            }

            Err(_timeout) => {
                tracing::error!(
                    "Task '{}' ({}) session {} timed out after {}s",
                    task_description,
                    task_id.as_string(),
                    session_id_clone,
                    timeout_secs
                );
                let _ = service::update_run_status(
                    &task_id,
                    Some(started_at),
                    Some(next_run_at),
                    None,
                    Some(TaskStatus::Failed.to_string()),
                    None,
                )
                .await;
                let _ = runs_service::finish_run(
                    &session_id_clone,
                    TaskStatus::Failed,
                    manager
                        .get_progress(&session_id_clone)
                        .map(progress_to_run_metrics),
                    Some(format!(
                        "Task exceeded max duration ({timeout_secs} seconds)"
                    )),
                )
                .await;
            }
        }

        abort_registry::remove(&session_id_clone);
    });

    abort_registry::register(&session_id, join_handle.abort_handle());
}
