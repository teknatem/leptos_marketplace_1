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
    task_type: "task017_wb_returns_claims",
    display_name: "WB Заявки на возврат (Claims)",
    description: "Загружает заявки покупателей на возврат товара из WB feedbacks-api \
        (/api/v1/claims). Опрашивает активные (is_archive=false) и архивные (is_archive=true) \
        заявки с пагинацией по 200 записей. API возвращает данные за последние 14 дней. \
        Уpsert по (connection_id, claim_id). Данные сохраняются в a032_wb_returns_claims.",
    external_apis: &[ExternalApiInfo {
        name: "WB Feedbacks API",
        base_url: "https://feedbacks-api.wildberries.ru/",
        rate_limit_desc: "20 запросов/мин; limit=200 на страницу",
    }],
    constraints: &[
        "Требует API-токена WB с категорией «Buyers Returns» (заявки покупателей на возврат)",
        "API возвращает только последние 14 дней — запускайте ежедневно или несколько раз в день",
        "Нет параметра даты — всегда загружается текущий срез активных и архивных заявок",
        "Создайте по одному экземпляру задачи для каждого WB-кабинета (sanstar, sts и т.д.)",
    ],
    config_fields: &[TaskConfigField {
        key: "connection_id",
        label: "WB Кабинет",
        hint: "Подключение к Wildberries из справочника «Подключения маркетплейсов». \
               API-токен должен иметь категорию «Buyers Returns».",
        field_type: TaskConfigFieldType::ConnectionMp,
        required: true,
        default_value: None,
        min_value: None,
        max_value: None,
    }],
    max_duration_seconds: 1800,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct Task017WbReturnsClaimsManager {
    executor: Arc<ImportExecutor>,
}

impl Task017WbReturnsClaimsManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task017WbReturnsClaimsManager {
    fn task_type(&self) -> &'static str {
        "task017_wb_returns_claims"
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
        logger.write_log(session_id, "task017: WB Returns Claims started")?;

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
            target_aggregates: vec!["a032_wb_returns_claims".to_string()],
            date_from: today,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task017: WB Returns Claims completed")?;
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
