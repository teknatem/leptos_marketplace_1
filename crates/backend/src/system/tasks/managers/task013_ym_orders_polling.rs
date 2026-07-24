use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use contracts::system::tasks::aggregate::ScheduledTask;
use contracts::system::tasks::metadata::{
    ExternalApiInfo, TaskConfigField, TaskConfigFieldType, TaskMetadata,
};
use contracts::system::tasks::progress::TaskProgress;
use contracts::usecases::u503_import_from_yandex::progress::ImportStatus;
use contracts::usecases::u503_import_from_yandex::request::{ImportMode, ImportRequest};
use serde::Deserialize;
use std::sync::Arc;

use crate::system::tasks::logger::TaskLogger;
use crate::system::tasks::manager::{TaskManager, TaskRunOutcome};
use crate::usecases::u503_import_from_yandex::ImportExecutor;

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task013_ym_orders_polling",
    write_tables: &["a013_ym_order", "a013_ym_order_items"],
    display_name: "YM Заказы — адаптивный поллер",
    description: "Каждые 5 минут загружает заказы Yandex Market через Partner API. \
        Фильтрует по дате ОБНОВЛЕНИЯ заказа (updatedAtFrom/updatedAtTo), поэтому ловит и новые \
        заказы, и смену статуса по ранее созданным. Работает в двух режимах: FAST (короткое окно \
        за сегодня и вчера, если последняя успешная загрузка была недавно) и EXTENDED (окно от \
        последней успешной загрузки с перекрытием при задержке или простое). Модель «подключение = бизнес»: если \
        у подключения задан БизнесАккаунтID, задание автоматически обходит все магазины \
        бизнеса через GET /campaigns; иначе — один магазин по supplier_id. placementType \
        кампании сохраняется в заказе как fulfillment_type. Данные сохраняются в a013_ym_order.",
    external_apis: &[ExternalApiInfo {
        name: "Yandex Market Partner API",
        base_url: "https://api.partner.market.yandex.ru/",
        rate_limit_desc: "Квоты зависят от кабинета и метода; принудительных задержек в задаче нет",
    }],
    constraints: &[
        "Требует подключение Yandex Market с API Key или OAuth 2.0 и campaign_id/supplier_id",
        "Фильтр по дате обновления (updatedAt); окно режется на под-интервалы ≤30 дней (лимит YM)",
        "FAST mode: date_from = сегодня-1, date_to = сегодня — минимальный объём данных",
        "EXTENDED mode: date_from = last_successful_run_at - overlap_minutes — покрывает период простоя",
        "Порог переключения FAST→EXTENDED: mode_threshold_minutes (по умолчанию 6 мин), считается от последней успешной загрузки",
        "overlap_minutes (по умолчанию 30) — запас назад от last_successful_run_at для надёжности",
        "fallback_lookback_hours (по умолчанию 24) — глубина при первом запуске",
        "БизнесАккаунтID задан — один прогон покрывает все магазины бизнеса (иначе только supplier_id)",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "Кабинет Яндекс Маркет",
            hint: "Подключение к Yandex Market Partner API из справочника «Подключения маркетплейсов»",
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
struct YmOrdersPollingConfig {
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

/// Адаптивный поллер заказов Yandex Market (task013).
/// Повторяет оконную стратегию task001 WB и запускает u503 только для a013_ym_order.
pub struct Task013YmOrdersPollingManager {
    executor: Arc<ImportExecutor>,
}

impl Task013YmOrdersPollingManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task013YmOrdersPollingManager {
    fn task_type(&self) -> &'static str {
        "task013_ym_orders_polling"
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
        let config: YmOrdersPollingConfig = serde_json::from_str(&task.config_json)?;

        let connection_id =
            super::config_helpers::parse_connection_id(&config.connection_id, "Яндекс Маркет")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Marketplace connection not found"))?;

        let now = Utc::now();
        let today = now.date_naive();
        let threshold = Duration::minutes(config.mode_threshold_minutes);

        // Якорь окна — последняя УСПЕШНАЯ загрузка (last_successful_run_at), а не последняя
        // попытка (last_run_at двигается и при ошибке). После серии ошибок восстановление
        // догоняет все обновления с момента последнего успеха (фильтр по updatedAt).
        let elapsed = task.last_successful_run_at.map(|last| now - last);
        let is_extended = elapsed.map_or(true, |e| e > threshold);

        let (date_from, mode_label) = if is_extended {
            let from_dt = task
                .last_successful_run_at
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

        logger.write_log(session_id, &format!("task013 mode: {mode_label}"))?;
        logger.write_log(
            session_id,
            &format!(
                "YM orders request window: {date_from} .. {today}; connection_id={}",
                config.connection_id
            ),
        )?;

        let request = ImportRequest {
            connection_id: config.connection_id.clone(),
            target_aggregates: vec!["a013_ym_order".to_string()],
            date_from,
            date_to: today,
            mode: ImportMode::Background,
            // Поллер синхронизирует статусы — фильтруем по дате обновления.
            // Перебор магазинов бизнеса выполняется автоматически в executor.
            incremental_by_update: true,
        };

        self.executor
            .execute_import(session_id, &request, &connection)
            .await?;

        logger.write_log(session_id, "task013 YM orders polling completed.")?;
        let completed_with_errors = self
            .executor
            .get_progress(session_id)
            .map(|p| {
                p.total_errors > 0
                    || matches!(
                        p.status,
                        ImportStatus::CompletedWithErrors | ImportStatus::Failed
                    )
            })
            .unwrap_or(false);
        if completed_with_errors {
            logger.write_log(
                session_id,
                "task013 completed with errors; see progress/errors for API diagnostics.",
            )?;
            return Ok(TaskRunOutcome::completed_with_errors());
        }

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
