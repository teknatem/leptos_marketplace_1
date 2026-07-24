use anyhow::{Context, Result};
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

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Config {
    connection_id: String,
    #[serde(default = "default_window_days")]
    window_days: i64,
}

fn default_window_days() -> i64 {
    7
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task020_wb_product_snapshot",
    write_tables: &["a037_wb_product_snapshot"],
    display_name: "WB Данные по товарам (остатки и рейтинги)",
    description:
        "Снимает текущее состояние товаров Wildberries (остатки на складах WB и продавца, \
        сумма остатков, рейтинг карточки и оценка покупателей) через Analytics API и сохраняет как \
        документ агрегата a037 за сегодняшнюю дату. Снимок только вперёд: историю задним числом WB \
        не отдаёт, поэтому запускать нужно ежедневно, чтобы строить динамику по дням.",
    external_apis: &[ExternalApiInfo {
        name: "WB Analytics API (sales-funnel/products)",
        base_url: "https://seller-analytics-api.wildberries.ru/",
        rate_limit_desc: "3 запроса/мин; постраничный проход с паузой 21 сек между страницами",
    }],
    constraints: &[
        "Требует API-токена WB с правами на Analytics API",
        "Охват — товары с активностью за окно window_days (по умолчанию 7 дней)",
        "Снимок сохраняется за текущую дату; повторный запуск за день перезаписывает документ",
        "Запускать ежедневно — пропущенный день восстановить нельзя",
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
            key: "window_days",
            label: "Окно активности (дней)",
            hint: "За сколько последних дней брать товары с активностью для снимка",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("7"),
            min_value: Some(1),
            max_value: Some(30),
        },
    ],
    max_duration_seconds: 7200,
};

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct Task020WbProductSnapshotManager {
    executor: Arc<ImportExecutor>,
}

impl Task020WbProductSnapshotManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task020WbProductSnapshotManager {
    fn task_type(&self) -> &'static str {
        "task020_wb_product_snapshot"
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
        logger.write_log(session_id, "task020: WB Product snapshot started")?;

        let cfg: Config = serde_json::from_str(&task.config_json).context(
            "Config parse failed — expected {\"connection_id\":\"<uuid>\",\"window_days\":7}",
        )?;

        let connection_id =
            super::config_helpers::parse_connection_id(&cfg.connection_id, "Wildberries")?;
        let connection = crate::domain::a006_connection_mp::service::get_by_id(connection_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Marketplace connection not found: {}", connection_id)
            })?;

        let window = cfg.window_days.clamp(1, 30);
        let today = Utc::now().naive_utc().date();
        let date_from = today - Duration::days(window - 1);
        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["a037_wb_product_snapshot".to_string()],
            date_from,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task020: WB Product snapshot completed")?;
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
