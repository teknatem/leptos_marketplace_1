use anyhow::{Context, Result};
use async_trait::async_trait;
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

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn default_work_start_date() -> String {
    "2026-01-01".to_string()
}
fn default_overlap_days() -> i64 {
    3
}
fn default_chunk_days() -> i64 {
    14
}

#[derive(Deserialize)]
struct Config {
    connection_id: String,
    #[serde(default = "default_work_start_date")]
    work_start_date: String,
    /// Late-arriving transactions appear with a delay — overlap accordingly
    #[serde(default = "default_overlap_days")]
    overlap_days: i64,
    #[serde(default = "default_chunk_days")]
    chunk_days: i64,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task019_ym_payment_report",
    display_name: "YM Отчёт по платежам",
    description: "Загружает отчёт по платежам Yandex Market через Partner API \
        (POST /v2/reports/united-netting/generate). Тяжёлый асинхронный процесс: \
        генерация → опрос статуса → скачивание ZIP → парсинг CSV → bulk-upsert. \
        Окно загрузки управляется watermark: грузит порциями chunk_days с перекрытием \
        overlap_days и догоняет до сегодня. Upsert идемпотентен по стабильному record_key, \
        повторная загрузка перекрывающихся окон безопасна. Данные сохраняются в \
        p907_ym_payment_report (+ проводки GL в p914).",
    external_apis: &[ExternalApiInfo {
        name: "Yandex Market Partner API (Reports)",
        base_url: "https://api.partner.market.yandex.ru/",
        rate_limit_desc: "Генерация отчёта асинхронная; опрос статуса до готовности, затем ZIP",
    }],
    constraints: &[
        "Требует подключение Yandex Market с API Key или OAuth 2.0 и business_account_id",
        "Тяжёлый асинхронный отчёт — рекомендуется ночной запуск",
        "Финансовые данные приходят с задержкой; overlap_days (по умолчанию 3) компенсирует это",
        "chunk_days (по умолчанию 14) — максимальный диапазон за один запуск",
        "Сброс watermark в UI запускает догон с work_start_date порциями chunk_days",
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
            key: "work_start_date",
            label: "Дата начала работы",
            hint: "Начиная с этой даты данные должны быть загружены полностью",
            field_type: TaskConfigFieldType::Date,
            required: false,
            default_value: Some("2026-01-01"),
            min_value: None,
            max_value: None,
        },
        TaskConfigField {
            key: "overlap_days",
            label: "Перекрытие от watermark (дн)",
            hint: "Запас назад от watermark для подхвата поздних транзакций",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("3"),
            min_value: Some(0),
            max_value: Some(14),
        },
        TaskConfigField {
            key: "chunk_days",
            label: "Размер порции (дн)",
            hint: "Максимальный диапазон за один запуск при догоняющей загрузке",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("14"),
            min_value: Some(1),
            max_value: Some(90),
        },
    ],
    max_duration_seconds: 7200,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

/// Регламентное задание загрузки отчёта по платежам Yandex Market (task019).
/// Watermark-стратегия по образцу task006 WB; запускает u503 только для p907_ym_payment_report.
pub struct Task019YmPaymentReportManager {
    executor: Arc<ImportExecutor>,
}

impl Task019YmPaymentReportManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task019YmPaymentReportManager {
    fn task_type(&self) -> &'static str {
        "task019_ym_payment_report"
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
        let cfg: Config = serde_json::from_str(&task.config_json)
            .context("Config parse failed — expected {\"connection_id\":\"<uuid>\",\"work_start_date\":\"2026-01-01\",\"overlap_days\":3,\"chunk_days\":14}")?;

        let connection_id =
            super::config_helpers::parse_connection_id(&cfg.connection_id, "Яндекс Маркет")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Marketplace connection not found: {}", connection_id)
            })?;

        let (date_from, date_to) = super::config_helpers::compute_date_window(
            task,
            &cfg.work_start_date,
            cfg.overlap_days,
            cfg.chunk_days,
        );

        logger.write_log(
            session_id,
            &format!(
                "task019 YM Payment Report: {} → {}; connection_id={}",
                date_from, date_to, cfg.connection_id
            ),
        )?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["p907_ym_payment_report".to_string()],
            date_from,
            date_to,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

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
                "task019 completed with errors; watermark NOT advanced — see progress/errors.",
            )?;
            return Ok(TaskRunOutcome::completed_with_errors());
        }

        logger.write_log(session_id, "task019: YM Payment Report completed")?;
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
