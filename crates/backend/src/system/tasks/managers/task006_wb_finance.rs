use anyhow::{Context, Result};
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

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

fn default_work_start_date() -> String {
    "2026-01-01".to_string()
}
fn default_overlap_days() -> i64 {
    1
}
fn default_chunk_days() -> i64 {
    7
}

#[derive(Deserialize)]
struct Config {
    connection_id: String,
    /// Max 31 (Statistics API hard limit per request)
    #[serde(default = "default_work_start_date")]
    work_start_date: String,
    #[serde(default = "default_chunk_days")]
    chunk_days: i64,
    /// Finance data appears 3-5 days after sale, overlap accordingly
    #[serde(default = "default_overlap_days")]
    overlap_days: i64,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task006_wb_finance",
    display_name: "WB Финансовый отчёт",
    description: "Загружает детальный финансовый отчёт из Wildberries через Statistics API \
        (/api/v5/supplier/reportDetailByPeriod). Запросы выполняются по одному на каждый день \
        периода из-за лимита 1 запрос/мин. Данные сохраняются в p903_wb_finance_report. \
        Финансовые данные появляются с задержкой 3–5 дней после продажи — overlap_days \
        компенсирует это смещение.",
    external_apis: &[ExternalApiInfo {
        name: "WB Statistics API",
        base_url: "https://statistics-api.wildberries.ru/",
        rate_limit_desc: "1 запрос/мин на /reportDetailByPeriod — принудительная пауза 65 сек",
    }],
    constraints: &[
        "Требует API-токена WB с доступом к Statistics API",
        "Жёсткий лимит: 1 запрос/мин; 14 дней = ~15 мин выполнения минимум",
        "Максимальный период за один запрос — 31 день",
        "Финансовые данные доступны с задержкой 3–5 дней после продажи",
        "overlap_days (по умолчанию 3) компенсирует задержку появления данных",
        "chunk_days (по умолчанию 7) — максимальный диапазон за один запуск",
        "Рекомендуется запускать 1 раз в день в ночное время",
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
            hint: "Запас назад от последнего запуска для надёжности границ",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("1"),
            min_value: Some(0),
            max_value: Some(7),
        },
        TaskConfigField {
            key: "chunk_days",
            label: "Размер порции (дн)",
            hint: "Максимальный диапазон за один запуск при догоняющей загрузке",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("7"),
            min_value: Some(1),
            max_value: Some(90),
        },
    ],
    max_duration_seconds: 14400,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct Task006WbFinanceManager {
    executor: Arc<ImportExecutor>,
}

impl Task006WbFinanceManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task006WbFinanceManager {
    fn task_type(&self) -> &'static str {
        "task006_wb_finance"
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
            .context("Config parse failed — expected {\"connection_id\":\"<uuid>\",\"work_start_date\":\"2026-01-01\",\"overlap_days\":3,\"chunk_days\":7}")?;

        let connection_id =
            super::config_helpers::parse_connection_id(&cfg.connection_id, "Wildberries")?;
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
                "task006 WB Finance: {} → {}  (1 req/min — ~{} min expected)",
                date_from,
                date_to,
                (date_to - date_from).num_days() + 1
            ),
        )?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["p903_wb_finance_report".to_string()],
            date_from,
            date_to,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task006: WB Finance report completed")?;
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
