use anyhow::Result;
use async_trait::async_trait;
use contracts::system::sys_scheduled_task::aggregate::ScheduledTask;
use contracts::system::sys_scheduled_task::progress::TaskProgress;
use contracts::usecases::u503_import_from_yandex::request::ImportRequest;
use std::sync::Arc;

use crate::system::sys_scheduled_task::logger::TaskLogger;
use crate::system::sys_scheduled_task::manager::TaskManager;
use crate::usecases::u503_import_from_yandex::ImportExecutor;

/// Менеджер для задачи импорта из Yandex Market (u503)
pub struct U503ImportYandexManager {
    executor: Arc<ImportExecutor>,
}

impl U503ImportYandexManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for U503ImportYandexManager {
    fn task_type(&self) -> &'static str {
        "u503_import_yandex"
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<()> {
        logger.write_log(session_id, "Starting U503 Import from Yandex Market...")?;

        let config: ImportRequest = serde_json::from_str(&task.config_json)?;

        // Get connection
        let connection_id = uuid::Uuid::parse_str(&config.connection_id)?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Marketplace connection not found"))?;

        self.executor
            .execute_import(session_id, &config, &connection)
            .await?;

        logger.write_log(session_id, "U503 Import from Yandex Market completed.")?;
        Ok(())
    }

    fn get_progress(&self, session_id: &str) -> Option<TaskProgress> {
        self.executor
            .progress_tracker
            .get_progress(session_id)
            .map(|p| p.into())
    }
}
