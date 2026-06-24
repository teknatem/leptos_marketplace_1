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
    2
}
fn default_chunk_days() -> i64 {
    14
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
    task_type: "task018_ym_returns",
    display_name: "YM Возвраты",
    description: "Загружает возвраты Yandex Market через Partner API \
        (GET /v2/campaigns/{campaignId}/returns). API фильтрует по дате ОБНОВЛЕНИЯ возврата \
        (fromDate/toDate = дата обновления), поэтому поздние изменения статусов сами попадают \
        в окно. Окно загрузки управляется watermark от последней успешной загрузки: грузит \
        порциями chunk_days дней с перекрытием overlap_days и догоняет до сегодня. \
        Upsert идемпотентен по return_id, поэтому повторная загрузка перекрывающихся окон \
        безопасна. Данные сохраняются в a016_ym_returns.",
    external_apis: &[ExternalApiInfo {
        name: "Yandex Market Partner API",
        base_url: "https://api.partner.market.yandex.ru/",
        rate_limit_desc: "Квоты зависят от кабинета и метода; пагинация по pageToken",
    }],
    constraints: &[
        "Требует подключение Yandex Market с API Key или OAuth 2.0 и campaign_id/supplier_id",
        "overlap_days (по умолчанию 2) компенсирует обновления статусов возвратов",
        "chunk_days (по умолчанию 14) — максимальный диапазон за один запуск",
        "Сброс watermark в UI запускает догон с work_start_date порциями chunk_days",
        "Рекомендуется запускать 1 раз в день",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "Кабинет Яндекс Маркет",
            hint:
                "Подключение к Yandex Market Partner API из справочника «Подключения маркетплейсов»",
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
            hint: "Запас назад от watermark для надёжности границ и обновлений статусов",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("2"),
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
    max_duration_seconds: 3600,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

/// Регламентное задание загрузки возвратов Yandex Market (task018).
/// Watermark-стратегия по образцу task006 WB; запускает u503 только для a016_ym_returns.
pub struct Task018YmReturnsManager {
    executor: Arc<ImportExecutor>,
}

impl Task018YmReturnsManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task018YmReturnsManager {
    fn task_type(&self) -> &'static str {
        "task018_ym_returns"
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
            .context("Config parse failed — expected {\"connection_id\":\"<uuid>\",\"work_start_date\":\"2026-01-01\",\"overlap_days\":2,\"chunk_days\":14}")?;

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
                "task018 YM Returns: {} → {}; connection_id={}",
                date_from, date_to, cfg.connection_id
            ),
        )?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["a016_ym_returns".to_string()],
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
                "task018 completed with errors; watermark NOT advanced — see progress/errors.",
            )?;
            return Ok(TaskRunOutcome::completed_with_errors());
        }

        logger.write_log(session_id, "task018: YM Returns completed")?;
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
