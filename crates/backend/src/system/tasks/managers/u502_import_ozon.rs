use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{
    ExternalApiInfo, TaskConfigField, TaskConfigFieldType, TaskMetadata,
};
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u502_import_from_ozon::request::{ImportMode, ImportRequest};
use serde::Deserialize;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};
use crate::usecases::u502_import_from_ozon::ImportExecutor;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn default_lookback_days() -> i64 {
    30
}

#[derive(Deserialize)]
struct Config {
    connection_id: String,
    #[serde(default = "default_lookback_days")]
    lookback_days: i64,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "u502_import_ozon",
    display_name: "Импорт из OZON",
    description: "Регулярный импорт всех агрегатов из кабинета OZON через Seller API: \
        товары (a007), продажи (a008), возвраты (a009), FBS-отправления (a010), \
        FBO-отправления (a011), транзакции (a014), финансовые отчёты (p902). \
        Дата начала периода: сегодня − lookback_days.",
    external_apis: &[ExternalApiInfo {
        name: "OZON Seller API",
        base_url: "https://api-seller.ozon.ru/",
        rate_limit_desc: "До 10 000 запросов/час на метод; задержка ≥ 0.5 с между батчами",
    }],
    constraints: &[
        "Требует Client-Id и Api-Key в конфигурации подключения (connection_id)",
        "Финансовые отчёты доступны с задержкой до 3 дней",
        "Пагинация через cursor/last_id; размер страницы зависит от метода",
        "Период за один запуск: сегодня − lookback_days … сегодня",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "Кабинет OZON",
            hint: "Подключение к OZON Seller API (Client-Id + Api-Key)",
            field_type: TaskConfigFieldType::ConnectionMp,
            required: true,
            default_value: None,
            min_value: None,
            max_value: None,
        },
        TaskConfigField {
            key: "lookback_days",
            label: "Глубина периода (дней)",
            hint: "date_from = сегодня − N дней; рекомендуется 30–60",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("30"),
            min_value: Some(1),
            max_value: Some(90),
        },
    ],
    max_duration_seconds: 7200,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

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

    fn metadata(&self) -> &'static TaskMetadata {
        &METADATA
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<TaskRunOutcome> {
        logger.write_log(session_id, "Starting U502 Import from OZON...")?;

        let cfg: Config = serde_json::from_str(&task.config_json).context(
            "Config parse failed — expected {\"connection_id\":\"<uuid>\",\"lookback_days\":30}",
        )?;

        let today = Utc::now().naive_utc().date();
        let date_from = today - Duration::days(cfg.lookback_days);

        logger.write_log(
            session_id,
            &format!(
                "Period: {} .. {}  (lookback_days={})",
                date_from, today, cfg.lookback_days
            ),
        )?;

        let connection_id = super::config_helpers::parse_connection_id(&cfg.connection_id, "Ozon")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Marketplace connection not found: {}", connection_id)
            })?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec![
                "a007_marketplace_product".to_string(),
                "a008_marketplace_sales".to_string(),
                "a009_ozon_returns".to_string(),
                "a010_ozon_fbs_posting".to_string(),
                "a011_ozon_fbo_posting".to_string(),
                "a014_ozon_transactions".to_string(),
                "p902_ozon_finance_realization".to_string(),
            ],
            mode: ImportMode::Background,
            date_from,
            date_to: today,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "U502 Import from OZON completed.")?;
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
