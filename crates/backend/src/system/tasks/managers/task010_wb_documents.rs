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
    #[serde(default = "default_work_start_date")]
    work_start_date: String,
    #[serde(default = "default_overlap_days")]
    overlap_days: i64,
    #[serde(default = "default_chunk_days")]
    chunk_days: i64,
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task010_wb_documents",
    display_name: "WB Документы",
    description: "Загружает список документов из Wildberries Documents API \
        (/api/v1/documents/list) за указанный период. \
        Данные сохраняются в a027_wb_documents.",
    external_apis: &[ExternalApiInfo {
        name: "WB Documents API",
        base_url: "https://documents-api.wildberries.ru/",
        rate_limit_desc: "1 запрос/10 с, burst 5; между страницами задержка 11 с",
    }],
    constraints: &[
        "Требует API-токена WB с правами на Documents API",
        "Между страницами — задержка 11 с (лимит 1 запрос/10 с, burst 5)",
        "overlap_days (по умолчанию 1) — перекрытие от watermark для надёжности",
        "chunk_days (по умолчанию 7) — максимальный диапазон за один запуск",
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
    max_duration_seconds: 7200,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct Task010WbDocumentsManager {
    executor: Arc<ImportExecutor>,
}

impl Task010WbDocumentsManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task010WbDocumentsManager {
    fn task_type(&self) -> &'static str {
        "task010_wb_documents"
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
            .context("Config parse failed — expected {\"connection_id\":\"<uuid>\",\"work_start_date\":\"2026-01-01\",\"overlap_days\":1,\"chunk_days\":7}")?;

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
            &format!("task010 WB Documents: {date_from} → {date_to}"),
        )?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["a027_wb_documents".to_string()],
            date_from,
            date_to,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task010: WB Documents completed")?;
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
