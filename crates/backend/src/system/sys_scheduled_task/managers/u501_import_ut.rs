use anyhow::Result;
use async_trait::async_trait;
use contracts::system::sys_scheduled_task::aggregate::ScheduledTask;
use contracts::system::sys_scheduled_task::progress::TaskProgress;
use contracts::usecases::u501_import_from_ut::request::ImportRequest;
use std::sync::Arc;

use crate::system::sys_scheduled_task::logger::TaskLogger;
use crate::system::sys_scheduled_task::manager::TaskManager;
use crate::usecases::u501_import_from_ut::ImportExecutor;

/// Менеджер для задачи импорта из УТ (u501)
pub struct U501ImportUtManager {
    executor: Arc<ImportExecutor>,
}

impl U501ImportUtManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for U501ImportUtManager {
    fn task_type(&self) -> &'static str {
        "u501_import_ut"
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<()> {
        logger.write_log(session_id, "Starting U501 Import from UT...")?;

        let config: ImportRequest = serde_json::from_str(&task.config_json)?;

        // Get connection
        let connection_id = uuid::Uuid::parse_str(&config.connection_id)?;
        let connection = crate::domain::a001_connection_1c::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connection 1C not found"))?;

        self.executor
            .execute_import(session_id, &config, &connection)
            .await?;

        logger.write_log(session_id, "U501 Import from UT completed.")?;
        Ok(())
    }

    fn get_progress(&self, session_id: &str) -> Option<TaskProgress> {
        self.executor
            .progress_tracker
            .get_progress(session_id)
            .map(|p| p.into())
    }
}
