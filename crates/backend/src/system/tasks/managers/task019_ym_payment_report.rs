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
fn default_resync_days() -> i64 {
    45
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
    /// Глубокий трейлинг-буфер дозагрузки (дни). В steady-state (когда задача догнала
    /// сегодня) каждый прогон перегружает последние N дней, чтобы поймать ретроактивно
    /// заполненные поля старых строк p907 (bank_order_id/act_id появляются спустя
    /// недели-месяцы). united-netting не имеет фильтра по дате изменения, поэтому только
    /// повторная загрузка с запасом ловит такие правки. 0 — отключить буфер.
    #[serde(default = "default_resync_days")]
    resync_days: i64,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task019_ym_payment_report",
    write_tables: &["p907_ym_payment_report"],
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
        "resync_days (по умолчанию 45): в steady-state каждый прогон перегружает последние N дней — \
         ловит ретроактивные правки старых строк (bank_order_id/act_id), т.к. в united-netting \
         нет фильтра по дате изменения",
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
        TaskConfigField {
            key: "resync_days",
            label: "Глубокий буфер дозагрузки (дн)",
            hint: "В steady-state каждый прогон перегружает последние N дней — ловит \
                   ретроактивно заполненные bank_order_id/act_id. 0 — отключить",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("45"),
            min_value: Some(0),
            max_value: Some(365),
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

        let (mut date_from, date_to) = super::config_helpers::compute_date_window(
            task,
            &cfg.work_start_date,
            cfg.overlap_days,
            cfg.chunk_days,
        );

        // Глубокий трейлинг-буфер: когда задача догнала сегодня (steady-state), всегда
        // перегружаем последние resync_days дней, чтобы поймать ретроактивно заполненные
        // поля (bank_order_id/act_id) старых строк. Во время порционного бэкфилла
        // (date_to < today) буфер не применяется — идёт обычная догрузка по chunk_days.
        let today = chrono::Utc::now().date_naive();
        if date_to >= today && cfg.resync_days > 0 {
            let work_start = chrono::NaiveDate::parse_from_str(&cfg.work_start_date, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
            let deep = (today - chrono::Duration::days(cfg.resync_days)).max(work_start);
            date_from = date_from.min(deep);
        }

        logger.write_log(
            session_id,
            &format!(
                "task019 YM Payment Report: {} → {}; connection_id={} (resync_days={})",
                date_from, date_to, cfg.connection_id, cfg.resync_days
            ),
        )?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["p907_ym_payment_report".to_string()],
            date_from,
            date_to,
            mode: ImportMode::Background,
            incremental_by_update: false,
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
