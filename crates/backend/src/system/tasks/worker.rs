use anyhow::Result;
use chrono::{DateTime, Utc};
use contracts::domain::common::AggregateId;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use super::task_session_runner::{spawn_task_session, TaskSessionParams};
use super::{logger::TaskLogger, registry::TaskManagerRegistry, runs_service, service};
use contracts::system::tasks::progress::TaskStatus;

/// Следующее время запуска по cron-расписанию.
fn next_run_from_cron(schedule_cron: &str, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let schedule = cron::Schedule::from_str(schedule_cron).ok()?;
    schedule.after(&after).next()
}

/// Фоновый воркер: каждые `interval_seconds` проверяет все включённые задачи
/// и запускает просроченные.
pub struct ScheduledTaskWorker {
    registry: Arc<TaskManagerRegistry>,
    logger: Arc<TaskLogger>,
    interval_seconds: u64,
}

impl ScheduledTaskWorker {
    pub fn new(
        registry: Arc<TaskManagerRegistry>,
        logger: Arc<TaskLogger>,
        interval_seconds: u64,
    ) -> Self {
        Self {
            registry,
            logger,
            interval_seconds,
        }
    }

    pub async fn run_loop(&self) {
        info!(
            "Scheduled task worker started (interval {}s)",
            self.interval_seconds
        );
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(self.interval_seconds));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            info!("Checking for due scheduled tasks…");
            if let Err(e) = self.process_due_tasks().await {
                tracing::error!("Error processing scheduled tasks: {:?}", e);
            }
        }
    }

    async fn process_due_tasks(&self) -> Result<()> {
        let now = Utc::now();
        let tasks = service::list_enabled_tasks().await?;

        for task in tasks {
            let should_run = task.next_run_at.map_or(true, |t| t <= now);
            if !should_run {
                continue;
            }

            let task_id_str = task.base.id.as_string();
            if let Ok(Some(_)) = runs_service::get_running_for_task(&task_id_str).await {
                warn!(
                    "Task '{}' ({}) is due but already running; skipping",
                    task.base.description, task_id_str
                );
                continue;
            }

            info!(
                "Task '{}' ({}) is due. Running…",
                task.base.description, task_id_str
            );

            let session_id = Uuid::new_v4().to_string();
            let task_id = task.base.id;

            let next_run = task
                .schedule_cron
                .as_deref()
                .and_then(|cron| next_run_from_cron(cron, now))
                .unwrap_or_else(|| now + chrono::Duration::hours(1));

            let log_file_path = self.logger.get_log_file_path(&session_id);

            if let Err(e) = runs_service::create_run_with_log(
                task_id,
                session_id.clone(),
                "Scheduled".to_string(),
                log_file_path.clone(),
            )
            .await
            {
                warn!("Failed to record run start for task {}: {}", task_id_str, e);
            }

            service::update_run_status(
                &task_id,
                Some(now),
                Some(next_run),
                Some(log_file_path),
                Some(TaskStatus::Running.to_string()),
                None,
            )
            .await?;

            spawn_task_session(TaskSessionParams {
                task,
                session_id,
                started_at: now,
                next_run_at: next_run,
                logger: Arc::clone(&self.logger),
                registry: Arc::clone(&self.registry),
            });
        }
        Ok(())
    }
}
