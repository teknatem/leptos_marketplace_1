use anyhow::Result;
use async_trait::async_trait;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u502_import_from_ozon::request::ImportRequest;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::TaskManager;
use crate::usecases::u502_import_from_ozon::ImportExecutor;

/// Менеджер для задачи импорта из OZON (u502)
pub struct U502ImportOzonManager {
    executor: Arc<ImportExecutor>,
}

impl U502ImportOzonManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for U502ImportOzonManager {
    fn task_type(&self) -> &'static str {
        "u502_import_ozon"
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<()> {
        logger.write_log(session_id, "Starting U502 Import from OZON...")?;

        let config: ImportRequest = serde_json::from_str(&task.config_json)?;

        // Get connection
        let connection_id = uuid::Uuid::parse_str(&config.connection_id)?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Marketplace connection not found"))?;

        self.executor
            .execute_import(session_id, &config, &connection)
            .await?;

        logger.write_log(session_id, "U502 Import from OZON completed.")?;
        Ok(())
    }

    fn get_progress(&self, session_id: &str) -> Option<TaskProgress> {
        self.executor
            .progress_tracker
            .get_progress(session_id)
            .map(|p| p.into())
    }
}
