use anyhow::Result;
use async_trait::async_trait;
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

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task002_wb_orders_stats_hourly",
    display_name: "WB Заказы — полная история (Statistics API)",
    description: "Загружает полные данные по заказам из Statistics API Wildberries \
        (/api/v1/supplier/orders). Содержит расширенные финансовые поля: forPay, finishedPrice, \
        привязки к поступлениям. Данные сохраняются в a015_wb_orders. \
        Загружает по chunk_days дней за запуск, постепенно заполняя историю от work_start_date.",
    external_apis: &[ExternalApiInfo {
        name: "WB Statistics API",
        base_url: "https://statistics-api.wildberries.ru/",
        rate_limit_desc: "1 запрос/мин — принудительная пауза 65 сек между страницами пагинации",
    }],
    constraints: &[
        "Требует API-токена WB с доступом к Statistics API (api_key_stats в подключении)",
        "Жёсткий лимит: 1 запрос/мин, 65 сек ожидания между страницами",
        "Максимальный период за один запрос — 31 день; chunk_days рекомендуется ≤ 31",
        "Рекомендуемый интервал запуска — не чаще 1 раза в час",
        "overlap_days (по умолчанию 1) — запас назад от watermark для устранения пробелов",
        "chunk_days (по умолчанию 30) — максимальный диапазон за один запуск при догоняющей загрузке",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "WB Кабинет",
            hint: "Подключение к Wildberries из справочника «Подключения маркетплейсов»",
            field_type: TaskConfigFieldType::ConnectionMp,
            required: true,
            default_value: None,
            min_value: None,
            max_value: None,
        },
        TaskConfigField {
            key: "work_start_date",
            label: "Дата начала работы",
            hint: "Начиная с этой даты данные должны быть загружены полностью. \
                   Сбросьте watermark в карточке задачи для перезагрузки с этой даты.",
            field_type: TaskConfigFieldType::Date,
            required: false,
            default_value: Some("2026-01-01"),
            min_value: None,
            max_value: None,
        },
        TaskConfigField {
            key: "overlap_days",
            label: "Перекрытие от watermark (дн)",
            hint: "Дополнительный запас назад от даты последнего запуска для надёжности границ периода",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("1"),
            min_value: Some(0),
            max_value: Some(7),
        },
        TaskConfigField {
            key: "chunk_days",
            label: "Размер порции (дн)",
            hint: "Максимальный диапазон за один запуск при догоняющей загрузке истории (≤ 31 дн)",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("7"),
            min_value: Some(1),
            max_value: Some(31),
        },
    ],
    max_duration_seconds: 14400,
};

#[derive(Debug, Deserialize)]
struct WbOrdersStatsHourlyConfig {
    connection_id: String,
    #[serde(default = "default_work_start_date")]
    work_start_date: String,
    #[serde(default = "default_overlap")]
    overlap_days: i64,
    #[serde(default = "default_chunk_days")]
    chunk_days: i64,
}

fn default_work_start_date() -> String {
    "2026-01-01".to_string()
}
fn default_overlap() -> i64 {
    1
}
fn default_chunk_days() -> i64 {
    30
}

/// Часовая загрузка полной истории заказов WB через Statistics API (task002).
/// Использует скользящее окно от последнего успешного запуска.
pub struct Task002WbOrdersStatsHourlyManager {
    executor: Arc<ImportExecutor>,
}

impl Task002WbOrdersStatsHourlyManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task002WbOrdersStatsHourlyManager {
    fn task_type(&self) -> &'static str {
        "task002_wb_orders_stats_hourly"
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
        let config: WbOrdersStatsHourlyConfig = serde_json::from_str(&task.config_json)?;

        let connection_id =
            super::config_helpers::parse_connection_id(&config.connection_id, "Wildberries")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Marketplace connection not found"))?;

        let (date_from, date_to) = super::config_helpers::compute_date_window(
            task,
            &config.work_start_date,
            config.overlap_days,
            config.chunk_days,
        );

        logger.write_log(
            session_id,
            &format!("task002 Statistics orders: {date_from} → {date_to}"),
        )?;

        let request = ImportRequest {
            connection_id: config.connection_id.clone(),
            target_aggregates: vec!["a015_wb_orders".to_string()],
            date_from,
            date_to,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &request, &connection)
            .await?;

        logger.write_log(session_id, "task002 WB orders Statistics completed.")?;
        Ok(TaskRunOutcome::completed_loaded_to(date_to))
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
