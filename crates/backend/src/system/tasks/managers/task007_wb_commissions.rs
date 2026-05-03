use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{
    ExternalApiInfo, TaskConfigField, TaskConfigFieldType, TaskMetadata,
};
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u504_import_from_wildberries::request::{ImportMode, ImportRequest};
use serde::Deserialize;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};
use crate::usecases::u504_import_from_wildberries::ImportExecutor;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Config {
    connection_id: String,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task007_wb_commissions",
    display_name: "WB Тарифы и комиссии",
    description: "Загружает актуальный снимок тарифов и комиссий WB из Common API \
        (/api/v1/tariffs/commission). Один запрос, без пагинации и дат — всегда текущий \
        тарифный план. Данные сохраняются в p905_wb_commission_history.",
    external_apis: &[ExternalApiInfo {
        name: "WB Common API",
        base_url: "https://common-api.wildberries.ru/",
        rate_limit_desc: "Без явных ограничений; один запрос на задачу",
    }],
    constraints: &[
        "Требует API-токена WB",
        "Тарифы изменяются редко — достаточно 1 раза в неделю",
        "Запрос возвращает текущий срез без истории изменений",
    ],
    config_fields: &[TaskConfigField {
        key: "connection_id",
        label: "WB Кабинет",
        hint: "Подключение к Wildberries из справочника «Подключения маркетплейсов»",
        field_type: TaskConfigFieldType::ConnectionMp,
        required: true,
        default_value: None,
        min_value: None,
        max_value: None,
    }],
    max_duration_seconds: 3600,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct Task007WbCommissionsManager {
    executor: Arc<ImportExecutor>,
}

impl Task007WbCommissionsManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task007WbCommissionsManager {
    fn task_type(&self) -> &'static str {
        "task007_wb_commissions"
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
        logger.write_log(session_id, "task007: WB Commissions snapshot started")?;

        let cfg: Config = serde_json::from_str(&task.config_json)
            .context("Config parse failed — expected {\"connection_id\":\"<uuid>\"}")?;

        let connection_id =
            super::config_helpers::parse_connection_id(&cfg.connection_id, "Wildberries")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Marketplace connection not found: {}", connection_id)
            })?;

        let today = Utc::now().naive_utc().date();
        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["p905_wb_commission_history".to_string()],
            date_from: today,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task007: WB Commissions completed")?;
        Ok(TaskRunOutcome::completed())
    }

    fn get_progress(&self, session_id: &str) -> Option<TaskProgress> {
        self.executor
            .progress_tracker
            .get_progress(session_id)
            .map(|p| p.into())
    }

    fn list_live_progress_sessions(&self) -> Vec<TaskProgress> {
        self.executor.list_live_task_progress()
    }
}
