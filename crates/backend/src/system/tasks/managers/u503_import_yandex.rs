use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{
    ExternalApiInfo, TaskConfigField, TaskConfigFieldType, TaskMetadata,
};
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u503_import_from_yandex::request::{ImportMode, ImportRequest};
use serde::Deserialize;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};
use crate::usecases::u503_import_from_yandex::ImportExecutor;

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
    task_type: "u503_import_yandex",
    display_name: "Импорт из Яндекс Маркет",
    description: "Регулярный импорт всех агрегатов из кабинета Яндекс Маркет через Partner API: \
        товары (a007), заказы (a013), возвраты (a016), финансовые отчёты (p907). \
        Дата начала периода: сегодня − lookback_days.",
    external_apis: &[ExternalApiInfo {
        name: "Яндекс Маркет Partner API",
        base_url: "https://api.partner.market.yandex.ru/",
        rate_limit_desc: "Стандартно до 600 запросов/мин на кабинет; квоты зависят от метода",
    }],
    constraints: &[
        "Требует OAuth-токена и campaign_id в конфигурации подключения",
        "Заказы доступны не ранее чем через 30 минут после создания в системе Яндекса",
        "Финансовые данные доступны с задержкой до 1 дня",
        "Период за один запуск: сегодня − lookback_days … сегодня",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "Кабинет Яндекс Маркет",
            hint: "Подключение к Яндекс Маркет Partner API (OAuth-токен + campaign_id)",
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

    fn metadata(&self) -> &'static TaskMetadata {
        &METADATA
    }

    async fn run(
        &self,
        task: &ScheduledTask,
        session_id: &str,
        logger: Arc<TaskLogger>,
    ) -> Result<TaskRunOutcome> {
        logger.write_log(session_id, "Starting U503 Import from Yandex Market...")?;

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

        let connection_id =
            super::config_helpers::parse_connection_id(&cfg.connection_id, "Яндекс Маркет")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Marketplace connection not found: {}", connection_id)
            })?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec![
                "a007_marketplace_product".to_string(),
                "a013_ym_order".to_string(),
                "a016_ym_returns".to_string(),
                "p907_ym_payment_report".to_string(),
            ],
            mode: ImportMode::Background,
            date_from,
            date_to: today,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "U503 Import from Yandex Market completed.")?;
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
