use anyhow::Result;
use chrono::{Duration, Utc};
use tokio::time::{self, MissedTickBehavior};
use tracing::{error, info, warn};
use uuid::Uuid;
use std::sync::Arc;
use contracts::domain::common::AggregateId;

use super::{
    logger::TaskLogger,
    registry::TaskManagerRegistry,
    service,
};
use contracts::system::tasks::progress::TaskStatus;

/// Фоновый воркер для выполнения запланированных задач.
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

    /// Запускает цикл выполнения задач.
    pub async fn run_loop(&self) {
        info!("Scheduled task worker started with interval {} seconds", self.interval_seconds);
        let mut interval = time::interval(time::Duration::from_secs(self.interval_seconds));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            info!("Checking for scheduled tasks to run...");
            if let Err(e) = self.process_due_tasks().await {
                error!("Error processing scheduled tasks: {:?}", e);
            }
        }
    }

    /// Обрабатывает задачи, время выполнения которых наступило.
    async fn process_due_tasks(&self) -> Result<()> {
        let now = Utc::now();
        let tasks = service::list_enabled_tasks().await?;

        for task in tasks {
            let should_run = match task.next_run_at {
                Some(next_run_at) => next_run_at <= now,
                None => true, // If next_run_at is not set, run it once or calculate it
            };

            if should_run {
                info!("Task '{}' ({}) is due. Running...", task.base.description, task.base.id.as_string());
                
                let session_id = Uuid::new_v4().to_string();
                let task_id = task.base.id;
                let task_type = task.task_type.clone();
                let task_description = task.base.description.clone();
                let task_logger = Arc::clone(&self.logger);
                let registry = Arc::clone(&self.registry);

                // Update next run time (simple logic for now: run again in 1 hour if not specified)
                // TODO: Implement actual cron parsing
                let next_run = now + Duration::hours(1);

                // Обновляем статус задачи перед запуском
                service::update_run_status(
                    &task_id,
                    Some(now),
                    Some(next_run),
                    Some(task_logger.get_log_file_path(&session_id)),
                    Some(TaskStatus::Running.to_string()),
                ).await?;

                let task_clone = task.clone();
                tokio::spawn(async move {
                    let manager = registry.get(&task_type);
                    match manager {
                        Some(mgr) => {
                            if let Err(e) = mgr.run(&task_clone, &session_id, task_logger).await {
                                error!("Task '{}' ({}) session {} failed: {:?}", task_description, task_id.as_string(), session_id, e);
                                let _ = service::update_run_status(
                                    &task_id,
                                    Some(now),
                                    Some(next_run),
                                    None, // keep existing log file path
                                    Some(TaskStatus::Failed.to_string()),
                                ).await;
                            } else {
                                info!("Task '{}' ({}) session {} completed successfully", task_description, task_id.as_string(), session_id);
                                let _ = service::update_run_status(
                                    &task_id,
                                    Some(now),
                                    Some(next_run),
                                    None, // keep existing log file path
                                    Some(TaskStatus::Completed.to_string()),
                                ).await;
                            }
                        }
                        None => {
                            warn!("No manager found for task type '{}' for task '{}' ({})", task_type, task_description, task_id.as_string());
                            let _ = service::update_run_status(
                                &task_id,
                                Some(now),
                                Some(next_run),
                                None, // keep existing log file path
                                Some(TaskStatus::Failed.to_string()),
                            ).await;
                        }
                    }
                });
            }
        }
        Ok(())
    }
}
