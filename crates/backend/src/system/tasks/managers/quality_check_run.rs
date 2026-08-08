use anyhow::Result;
use async_trait::async_trait;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::TaskMetadata;
use contracts::system::tasks::progress::TaskProgress;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "quality_check_run",
    display_name: "Контроль качества — запуск проверки",
    description: "Запускает любое зарегистрированное quality check по id; MJS-правила не требуют отдельного task manager.",
    external_apis: &[],
    constraints: &["config_json: {\"check_id\":\"...\",\"input\":{...}}", "Проверки MJS имеют только read-only доступ к объявленным таблицам"],
    write_tables: &["sys_quality_check_runs"],
    config_fields: &[],
    max_duration_seconds: 120,
};

#[derive(Debug, Deserialize)]
struct Config {
    check_id: String,
    #[serde(default)]
    input: Value,
}

pub struct QualityCheckRunManager;

impl QualityCheckRunManager {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl TaskManager for QualityCheckRunManager {
    fn task_type(&self) -> &'static str {
        "quality_check_run"
    }
    fn metadata(&self) -> &'static TaskMetadata {
        &METADATA
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<TaskRunOutcome> {
        let config: Config = serde_json::from_str(&task.config_json)
            .map_err(|error| anyhow::anyhow!("Invalid quality_check_run config: {error}"))?;
        if config.check_id.trim().is_empty() {
            anyhow::bail!("quality_check_run.check_id is required");
        }
        logger.write_log(
            session_id,
            &format!("Quality check '{}' started", config.check_id),
        )?;
        let details = crate::quality::run_check_with_input(
            &config.check_id,
            config.input,
            &format!("scheduled:{session_id}"),
        )
        .await?;
        logger.write_log(
            session_id,
            &format!(
                "Quality check '{}' completed: population={}, violations={}",
                config.check_id, details.result.population_total, details.result.violations_total
            ),
        )?;
        Ok(TaskRunOutcome::completed())
    }

    fn get_progress(&self, _session_id: &str) -> Option<TaskProgress> {
        None
    }
}
