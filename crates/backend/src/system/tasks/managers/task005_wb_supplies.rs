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

fn default_work_start_date() -> String {
    "2026-01-01".to_string()
}
fn default_overlap_days() -> i64 {
    1
}
fn default_chunk_days() -> i64 {
    7
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

static METADATA: TaskMetadata = TaskMetadata {
    task_type: "task005_wb_supplies",
    display_name: "WB РџРѕСЃС‚Р°РІРєРё FBS",
    description: "Р—Р°РіСЂСѓР¶Р°РµС‚ РїРѕСЃС‚Р°РІРєРё FBS РёР· Wildberries Рё РїСЂРёРІСЏР·С‹РІР°РµС‚ РёС… Рє Р·Р°РєР°Р·Р°Рј. \
        РџРѕСЃР»РµРґРѕРІР°С‚РµР»СЊРЅРѕСЃС‚СЊ РЅРµСЂР°Р·СЂС‹РІРЅР°: СЃРїРёСЃРѕРє РїРѕСЃС‚Р°РІРѕРє в†’ ID Р·Р°РєР°Р·РѕРІ РїРѕ РєР°Р¶РґРѕР№ РїРѕСЃС‚Р°РІРєРµ в†’ \
        СЃС‚РёРєРµСЂС‹ Р±Р°С‚С‡Р°РјРё РґРѕ 1000 С€С‚. в†’ СЃРёРЅС…СЂРѕРЅРёР·Р°С†РёСЏ income_id РІ a015_wb_orders. \
        Р”Р°РЅРЅС‹Рµ СЃРѕС…СЂР°РЅСЏСЋС‚СЃСЏ РІ a029_wb_supply.",
    external_apis: &[ExternalApiInfo {
        name: "WB Seller API (Marketplace)",
        base_url: "https://marketplace-api.wildberries.ru/",
        rate_limit_desc: "300 Р·Р°РїСЂРѕСЃРѕРІ/РјРёРЅ; РѕС‚РґРµР»СЊРЅС‹Р№ Р±Р°С‚С‡ РЅР° СЃС‚РёРєРµСЂС‹ (РґРѕ 1000 С€С‚.)",
    }],
    constraints: &[
        "РўСЂРµР±СѓРµС‚ API-С‚РѕРєРµРЅР° WB СЃ РїСЂР°РІР°РјРё РЅР° FBS-РїРѕСЃС‚Р°РІРєРё",
        "РЎС‚РёРєРµСЂС‹ Р·Р°РїСЂР°С€РёРІР°СЋС‚СЃСЏ С‚РѕР»СЊРєРѕ РґР»СЏ РїРѕСЃС‚Р°РІРѕРє РІ СЃС‚Р°С‚СѓСЃРµ РѕР¶РёРґР°РЅРёСЏ СЃР±РѕСЂРєРё",
        "РЎРёРЅС…СЂРѕРЅРёР·РёСЂСѓРµС‚ income_id РІ a015_wb_orders вЂ” Р·Р°РґРµР№СЃС‚РІСѓРµС‚ РґРІР° Р°РіСЂРµРіР°С‚Р°, РЅРѕ РѕРїРµСЂР°С†РёСЏ РЅРµСЂР°Р·СЂС‹РІРЅР°",
        "chunk_days (по умолчанию 7) — максимальный диапазон за один запуск",
    ],
    config_fields: &[
        TaskConfigField {
            key: "connection_id",
            label: "WB РљР°Р±РёРЅРµС‚",
            hint: "РџРѕРґРєР»СЋС‡РµРЅРёРµ Рє Wildberries РёР· СЃРїСЂР°РІРѕС‡РЅРёРєР° В«РџРѕРґРєР»СЋС‡РµРЅРёСЏ РјР°СЂРєРµС‚РїР»РµР№СЃРѕРІВ»",
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

pub struct Task005WbSuppliesManager {
    executor: Arc<ImportExecutor>,
}

impl Task005WbSuppliesManager {
    pub fn new(executor: Arc<ImportExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl TaskManager for Task005WbSuppliesManager {
    fn task_type(&self) -> &'static str {
        "task005_wb_supplies"
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
        let cfg: Config = serde_json::from_str(&task.config_json).context(
            "Config parse failed — expected {\"connection_id\":\"<uuid>\",\"work_start_date\":\"2026-01-01\",\"overlap_days\":1,\"chunk_days\":7}"
        )?;

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
            &format!("task005 WB Supplies: {date_from} -> {date_to}"),
        )?;

        let req = ImportRequest {
            connection_id: cfg.connection_id,
            target_aggregates: vec!["a029_wb_supply".to_string()],
            date_from,
            date_to,
            mode: ImportMode::Background,
        };

        self.executor
            .execute_import(session_id, &req, &connection)
            .await?;

        logger.write_log(session_id, "task005: WB Supplies completed")?;
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
