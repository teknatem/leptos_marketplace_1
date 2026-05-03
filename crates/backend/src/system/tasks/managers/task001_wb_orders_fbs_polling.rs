use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
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
    task_type: "task001_wb_orders_fbs_polling",
    display_name: "WB Заказы FBS — адаптивный поллер",
    description: "Каждые 5 минут загружает новые заказы FBS из Wildberries через Marketplace API. \
        Работает в двух режимах: FAST (только /orders/new + /orders за текущий день — если последний \
        запуск был менее mode_threshold_minutes назад) и EXTENDED (/orders/new + /orders за окно от \
        последнего запуска — при задержке или простое сервера). Данные сохраняются в a015_wb_orders_new.",
    external_apis: &[ExternalApiInfo {
        name: "WB Seller API (Marketplace)",
        base_url: "https://marketplace-api.wildberries.ru/",
        rate_limit_desc: "300 запросов/мин; нет принудительных задержек между страницами пагинации",
    }],
    constraints: &[
        "Требует API-токена WB с правами на FBS-заказы (connection_id в config_json)",
        "FAST mode: date_from = сегодня-1, date_to = сегодня — минимальный объём данных",
        "EXTENDED mode: date_from = last_run_at - overlap_minutes — покрывает весь период простоя",
        "Порог переключения FAST→EXTENDED: mode_threshold_minutes (по умолчанию 6 мин)",
        "overlap_minutes (по умолчанию 30) — запас назад от last_run_at для надёжности",
        "fallback_lookback_hours (по умолчанию 24) — глубина при первом запуске (нет last_run_at)",
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
            key: "mode_threshold_minutes",
            label: "Порог переключения в EXTENDED (мин)",
            hint: "Если прошло больше N минут с последнего запуска — используется расширенный режим с историческим окном",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("6"),
            min_value: Some(1),
            max_value: Some(1440),
        },
        TaskConfigField {
            key: "fallback_lookback_hours",
            label: "Глубина при первом запуске (ч)",
            hint: "Количество часов назад при отсутствии истории выполнений (первый запуск)",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("24"),
            min_value: Some(1),
            max_value: Some(168),
        },
        TaskConfigField {
            key: "overlap_minutes",
            label: "Перекрытие от last_run_at (мин)",
            hint: "Дополнительный запас назад от времени последнего запуска для надёжности границ",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("30"),
            min_value: Some(0),
            max_value: Some(1440),
        },
    ],
    max_duration_seconds: 1800,
};

#[derive(Debug, Deserialize)]
struct WbOrdersFbsPollingConfig {
    connection_id: String,
    #[serde(default = "default_threshold")]
    mode_threshold_minutes: i64,
    #[serde(default = "default_fallback_hours")]
    fallback_lookback_hours: i64,
    #[serde(default = "default_overlap")]
    overlap_minutes: i64,
}

fn default_threshold() -> i64 {
    6
}
fn default_fallback_hours() -> i64 {
    24
}
fn default_overlap() -> i64 {
    30
}

/// Адаптивный поллер FBS-заказов WB (task001).
/// Переключается между FAST и EXTENDED режимами по времени с последнего запуска.
pub struct Task001WbOrdersFbsPollingManager {
    executor: Arc<ImportExecutor>,
}

impl Task001WbOrdersFbsPollingManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task001WbOrdersFbsPollingManager {
    fn task_type(&self) -> &'static str {
        "task001_wb_orders_fbs_polling"
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
        let config: WbOrdersFbsPollingConfig = serde_json::from_str(&task.config_json)?;

        let connection_id =
            super::config_helpers::parse_connection_id(&config.connection_id, "Wildberries")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Marketplace connection not found"))?;

        let now = Utc::now();
        let today = now.date_naive();
        let threshold = Duration::minutes(config.mode_threshold_minutes);

        let elapsed = task.last_run_at.map(|last| now - last);
        let is_extended = elapsed.map_or(true, |e| e > threshold);

        let (date_from, mode_label) = if is_extended {
            let from_dt = task
                .last_run_at
                .map(|last| last - Duration::minutes(config.overlap_minutes))
                .unwrap_or_else(|| now - Duration::hours(config.fallback_lookback_hours));
            let from_date = from_dt.date_naive();
            let gap_desc = elapsed
                .map(|e| format!("{} мин", e.num_minutes()))
                .unwrap_or_else(|| "первый запуск".to_string());
            (
                from_date,
                format!("EXTENDED (gap: {gap_desc}, from: {from_date})"),
            )
        } else {
            (today - Duration::days(1), "FAST".to_string())
        };

        logger.write_log(session_id, &format!("task001 mode: {mode_label}"))?;

        let request = ImportRequest {
            connection_id: config.connection_id.clone(),
            target_aggregates: vec!["a015_wb_orders_new".to_string()],
            date_from,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &request, &connection)
            .await?;

        logger.write_log(session_id, "task001 WB orders FBS polling completed.")?;
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
