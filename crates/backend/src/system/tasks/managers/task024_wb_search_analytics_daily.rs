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

#[derive(Deserialize)]
struct Config {
    connection_id: String,
    #[serde(default = "default_window_days")]
    window_days: i64,
}

fn default_window_days() -> i64 {
    1
}

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task024_wb_search_analytics_daily",
    write_tables: &["a040_wb_search_analytics_daily"],
    display_name: "WB Поисковая аналитика (показы)",
    description:
        "Загружает поисковую аналитику Wildberries (органические показы, переходы из поиска, \
        средняя позиция в выдаче, видимость + топ поисковых запросов на товар) через \
        search-report API и сохраняет как документы агрегата a040 — один документ на дату, \
        строка на nm_id. Показы питают верх воронки p916 (show_free_count). Требует подписки «Джем». \
        Данные forward-only — запускать регулярно.",
    external_apis: &[ExternalApiInfo {
        name: "WB Analytics API (search-report/report + product/search-texts)",
        base_url: "https://seller-analytics-api.wildberries.ru/",
        rate_limit_desc: "лимиты аналитики WB; топ-запросы чанками по 20 nm_id с паузой",
    }],
    constraints: &[
        "Требует API-токена WB с правами на Analytics API и подписки «Джем»",
        "При отсутствии доступа/подписки (403) задача логирует и завершается без ошибки",
        "Данные за дату перезаписываются целиком (replace_for_period)",
        "Не запускать одновременно с task020/task023 по тому же кабинету — общий лимит кабинета",
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
            hint: "За сколько последних дней грузить (снимок делается за конечную дату окна)",
            field_type: TaskConfigFieldType::Integer,
            required: false,
            default_value: Some("1"),
            min_value: Some(1),
            max_value: Some(30),
        },
    ],
    max_duration_seconds: 7200,
};

pub struct Task024WbSearchAnalyticsDailyManager {
    executor: Arc<ImportExecutor>,
}

impl Task024WbSearchAnalyticsDailyManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task024WbSearchAnalyticsDailyManager {
    fn task_type(&self) -> &'static str {
        "task024_wb_search_analytics_daily"
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
        logger.write_log(session_id, "task024: WB Search analytics started")?;

        let cfg: Config = serde_json::from_str(&task.config_json).context(
            "Config parse failed — expected {\"connection_id\":\"<uuid>\",\"window_days\":1}",
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
            target_aggregates: vec!["a040_wb_search_analytics_daily".to_string()],
            date_from,
            date_to: today,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task024: WB Search analytics completed")?;
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
