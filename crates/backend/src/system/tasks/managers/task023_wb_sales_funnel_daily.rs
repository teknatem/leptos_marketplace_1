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
    task_type: "task023_wb_sales_funnel_daily",
    display_name: "WB Воронка продаж (по дням)",
    description:
        "Загружает воронку продаж Wildberries по дням (переходы в карточку, добавления в корзину, \
        заказы, выкупы, конверсии) через Analytics API и сохраняет как документы агрегата a036 — \
        один документ на дату, строка на nm_id. WB хранит воронку примерно за последнюю неделю, \
        поэтому запускать нужно регулярно: данные старше окна хранения восстановить нельзя.",
    external_apis: &[ExternalApiInfo {
        name: "WB Analytics API (sales-funnel/products + products/history)",
        base_url: "https://seller-analytics-api.wildberries.ru/",
        rate_limit_desc: "3 запроса/мин на метод; пауза 21 сек между вызовами, история чанками по 20 nm_id",
    }],
    constraints: &[
        "Требует API-токена WB с правами на Analytics API",
        "WB отдаёт воронку примерно за последнюю неделю — окно больше 7 дней вернёт пустые дни",
        "Данные за окно перезаписываются целиком (replace_for_period)",
        "Прогон длительный: 21 сек между вызовами, несколько тысяч товаров — десятки минут",
        "Не запускать одновременно с task020 по тому же кабинету — общий лимит 3 запроса/мин",
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
            label: "Окно загрузки (дней)",
            hint: "За сколько последних дней грузить воронку. WB хранит ~7 дней — больше ставить бессмысленно",
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

pub struct Task023WbSalesFunnelDailyManager {
    executor: Arc<ImportExecutor>,
}

impl Task023WbSalesFunnelDailyManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task023WbSalesFunnelDailyManager {
    fn task_type(&self) -> &'static str {
        "task023_wb_sales_funnel_daily"
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
        logger.write_log(session_id, "task023: WB Sales funnel started")?;

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
            target_aggregates: vec!["a036_wb_sales_funnel_daily".to_string()],
            date_from,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task023: WB Sales funnel completed")?;
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
