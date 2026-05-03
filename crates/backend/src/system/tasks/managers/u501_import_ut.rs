use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{
    ExternalApiInfo, TaskConfigField, TaskConfigFieldType, TaskMetadata,
};
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u501_import_from_ut::request::{ImportMode, ImportRequest};
use serde::Deserialize;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};
use crate::usecases::u501_import_from_ut::ImportExecutor;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn default_lookback_days() -> i64 {
    30
}

#[derive(Deserialize)]
struct Config {
    /// UUID из таблицы a001_connection_1c
    connection_id: String,
    #[serde(default = "default_lookback_days")]
    lookback_days: i64,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "u501_import_ut",
    display_name: "Импорт из 1С:Управление торговлей",
    description: "Регулярный импорт справочников и документов из базы 1С:УТ11 через OData API: \
        организации (a002), контрагенты (a003), номенклатура (a004), \
        комплектации (a022), закупки (a023), штрихкоды (p901), цены (p906). \
        Период ценовых загрузок: сегодня − lookback_days … сегодня.",
    external_apis: &[ExternalApiInfo {
        name: "1С:УТ11 OData API",
        base_url: "http://<server>/UT11/odata/standard.odata/",
        rate_limit_desc:
            "Определяется конфигурацией сервера 1С; рекомендуется ≤ 5 параллельных запросов",
    }],
    constraints: &[
        "Требует активного подключения к базе 1С (connection_id → a001_connection_1c)",
        "OData-сессия привязана к учётным данным пользователя 1С",
        "Большие выборки (> 10 000 строк) разбиваются на страницы по $top/$skip",
        "Период ценовых данных (p906): сегодня − lookback_days … сегодня",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "Подключение к 1С",
            hint: "UUID подключения из таблицы a001_connection_1c (база 1С:УТ11)",
            field_type: TaskConfigFieldType::Text,
            required: true,
            default_value: None,
            min_value: None,
            max_value: None,
        },
        TaskConfigField {
            key: "lookback_days",
            label: "Глубина периода (дней)",
            hint: "Период для загрузки ценовых данных (p906): сегодня − N дней",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("30"),
            min_value: Some(1),
            max_value: Some(365),
        },
    ],
    max_duration_seconds: 7200,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

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

    fn metadata(&self) -> &'static TaskMetadata {
        &METADATA
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<TaskRunOutcome> {
        logger.write_log(session_id, "Starting U501 Import from 1С UT...")?;

        let cfg: Config = serde_json::from_str(&task.config_json).context(
            "Config parse failed — expected {\"connection_id\":\"<uuid>\",\"lookback_days\":30}",
        )?;

        let today = Utc::now().naive_utc().date();
        let period_from = today - Duration::days(cfg.lookback_days);

        logger.write_log(
            session_id,
            &format!(
                "Period for prices: {} .. {}  (lookback_days={})",
                period_from, today, cfg.lookback_days
            ),
        )?;

        let connection_id = super::config_helpers::parse_connection_id(&cfg.connection_id, "1С")?;
        let connection = crate::domain::a001_connection_1c::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connection 1C not found: {}", connection_id))?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec![
                "a002_organization".to_string(),
                "a003_counterparty".to_string(),
                "a004_nomenclature".to_string(),
                "a022_kit_variant".to_string(),
                "a023_purchase_of_goods".to_string(),
                "p901_barcodes".to_string(),
                "p906_prices".to_string(),
            ],
            mode: ImportMode::Background,
            delete_obsolete: false,
            period_from: Some(period_from.to_string()),
            period_to: Some(today.to_string()),
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "U501 Import from 1С UT completed.")?;
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
