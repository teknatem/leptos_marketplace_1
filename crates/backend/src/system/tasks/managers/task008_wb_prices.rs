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
    task_type: "task008_wb_prices",
    display_name: "WB Цены и скидки",
    description: "Загружает актуальные розничные цены и скидки из Wildberries Prices API \
        (/api/v2/list/goods/filter). Пагинация по cursor, данные обогащаются \
        внутренними ссылками на номенклатуру. Дата не нужна — всегда актуальный срез. \
        Данные сохраняются в p908_wb_goods_prices.",
    external_apis: &[ExternalApiInfo {
        name: "WB Prices & Discounts API",
        base_url: "https://discounts-prices-api.wildberries.ru/",
        rate_limit_desc: "300 запросов/мин; пагинация через cursor",
    }],
    constraints: &[
        "Требует API-токена WB с правами на Prices API",
        "Загружает текущие цены — историчность не поддерживается",
        "Рекомендуется запускать 1–2 раза в день",
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
    max_duration_seconds: 7200,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct Task008WbPricesManager {
    executor: Arc<ImportExecutor>,
}

impl Task008WbPricesManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task008WbPricesManager {
    fn task_type(&self) -> &'static str {
        "task008_wb_prices"
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
        logger.write_log(session_id, "task008: WB Prices sync started")?;

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
            target_aggregates: vec!["p908_wb_goods_prices".to_string()],
            date_from: today,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task008: WB Prices completed")?;
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
