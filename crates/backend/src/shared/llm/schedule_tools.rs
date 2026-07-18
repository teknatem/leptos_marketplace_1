//! LLM-инструменты по регламентным заданиям (планировщик, `sys_tasks`).
//!
//! Дополняют `admin_tools`: там инструменты run-центричные (история прогонов),
//! здесь — расписание, ватермарки и статические метаданные типов заданий,
//! то есть материал для консультаций и рекомендаций.
//!
//! Только чтение: изменение расписаний и включение/выключение — через UI.

use super::types::ToolDefinition;
use crate::system::tasks::{registry, runs_service, service};
use contracts::system::tasks::metadata::TaskMetadataDto;
use serde_json::{json, Value};

/// Имена инструментов навыка (для маршрутизации в `execute_tool_call`).
pub const SCHEDULE_TOOL_NAMES: &[&str] = &["list_scheduled_tasks", "describe_task_types"];

/// Планировщик считает cron в UTC (`worker.rs` использует `Utc::now()`).
/// Без этой оговорки агент будет читать и рекомендовать время в неверном поясе.
const TZ_HINT: &str = "ВАЖНО: cron и все отметки времени — в UTC. МСК = UTC+3, \
                       то есть '0 0 3,15 * * *' — это 06:00 и 18:00 по Москве.";

pub fn schedule_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_scheduled_tasks".into(),
            description: "Список регламентных заданий планировщика с их расписанием и \
                          состоянием: cron, включено ли, время следующего и последнего \
                          запуска, статус, дата последнего успеха, ватермарка загруженных \
                          данных и параметры (config_json). Отвечает на вопросы «что \
                          настроено», «когда обновлялись данные», «почему данных нет». \
                          Для описания того, ЧТО делает задание и какие у него ограничения, \
                          вызови describe_task_types."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "include_disabled": {
                        "type": "boolean",
                        "description": "Включать выключенные задания (по умолчанию true — \
                                        выключенное задание часто и есть причина отсутствия данных)."
                    },
                    "task_code": {
                        "type": "string",
                        "description": "Фильтр по коду или типу задания, подстрока без учёта \
                                        регистра, например 'wb' или 'task023'. Пусто — все."
                    },
                    "runs_limit": {
                        "type": "integer",
                        "description": "Сколько последних прогонов показать по каждому заданию (0–10, по умолчанию 3).",
                        "minimum": 0,
                        "maximum": 10
                    }
                }
            }),
        },
        ToolDefinition {
            name: "describe_task_types".into(),
            description: "Справка по типам регламентных заданий из реестра планировщика: \
                          что задание делает, какие внешние API дёргает и с какими лимитами, \
                          ограничения (constraints), схема параметров и максимальная \
                          длительность прогона. Используй, чтобы объяснить поведение задания \
                          или обосновать рекомендацию по частоте запуска."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "task_type": {
                        "type": "string",
                        "description": "Точный тип задания, например 'task023_wb_sales_funnel_daily'. \
                                        Пусто — вернуть все зарегистрированные типы."
                    }
                }
            }),
        },
    ]
}

/// Выполнить tool call планировщика. Возвращает JSON-результат.
pub async fn execute_schedule_tool(name: &str, arguments_json: &str) -> Value {
    let args: Value = serde_json::from_str(arguments_json).unwrap_or_else(|_| json!({}));

    match name {
        "list_scheduled_tasks" => {
            let include_disabled = args
                .get("include_disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let filter = args
                .get("task_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase());
            let runs_limit = args
                .get("runs_limit")
                .and_then(|v| v.as_i64())
                .unwrap_or(3)
                .clamp(0, 10) as u64;
            list_scheduled_tasks(include_disabled, filter, runs_limit).await
        }
        "describe_task_types" => {
            let task_type = args.get("task_type").and_then(|v| v.as_str());
            describe_task_types(task_type)
        }
        other => json!({ "ok": false, "error": format!("Неизвестный инструмент планировщика: {other}") }),
    }
}

async fn list_scheduled_tasks(
    include_disabled: bool,
    filter: Option<String>,
    runs_limit: u64,
) -> Value {
    let tasks = match service::list_all().await {
        Ok(t) => t,
        Err(e) => return json!({ "ok": false, "error": format!("Ошибка чтения sys_tasks: {e}") }),
    };

    let mut items = Vec::new();
    for task in tasks {
        if !include_disabled && !task.is_enabled {
            continue;
        }
        if let Some(f) = &filter {
            let code = task.base.code.to_lowercase();
            let ttype = task.task_type.to_lowercase();
            if !code.contains(f.as_str()) && !ttype.contains(f.as_str()) {
                continue;
            }
        }

        let task_id = task.base.id.0.to_string();
        let recent_runs: Vec<Value> = if runs_limit == 0 {
            Vec::new()
        } else {
            runs_service::list_for_task(&task_id, runs_limit)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|r| {
                    json!({
                        "started_at": r.started_at,
                        "status": r.status,
                        "triggered_by": r.triggered_by,
                        "duration_ms": r.duration_ms,
                        "total_processed": r.total_processed,
                        "total_errors": r.total_errors,
                        "error_message": r.error_message,
                    })
                })
                .collect()
        };

        items.push(json!({
            "code": task.base.code,
            "description": task.base.description,
            "task_type": task.task_type,
            "schedule_cron": task.schedule_cron,
            "is_enabled": task.is_enabled,
            "next_run_at": task.next_run_at,
            "last_run_at": task.last_run_at,
            "last_run_status": task.last_run_status,
            "last_successful_run_at": task.last_successful_run_at,
            "data_loaded_up_to": task.data_loaded_up_to,
            "config_json": task.config_json,
            "recent_runs": recent_runs,
        }));
    }

    let total = items.len();
    let disabled = items
        .iter()
        .filter(|i| i.get("is_enabled").and_then(|v| v.as_bool()) == Some(false))
        .count();

    json!({
        "ok": true,
        "total": total,
        "disabled_count": disabled,
        "tasks": items,
        "hint": format!(
            "{TZ_HINT} Задание с is_enabled=false не запускается вообще — это первая гипотеза, \
             если данных нет. data_loaded_up_to — дата, по которую данные загружены включительно. \
             Пустой schedule_cron означает запуск только вручную. Что именно делает задание и \
             почему нельзя запускать чаще — в describe_task_types(task_type)."
        ),
    })
}

fn describe_task_types(task_type: Option<&str>) -> Value {
    let Some(registry) = registry::get_global_registry() else {
        return json!({ "ok": false, "error": "Реестр задач не инициализирован" });
    };

    let mut metas: Vec<TaskMetadataDto> = registry
        .list_metadata()
        .into_iter()
        .filter(|m| task_type.is_none_or(|t| m.task_type == t))
        .map(TaskMetadataDto::from)
        .collect();
    metas.sort_by(|a, b| a.task_type.cmp(&b.task_type));

    if metas.is_empty() {
        return json!({
            "ok": false,
            "error": format!("Тип задания не найден: {}", task_type.unwrap_or("")),
            "hint": "Список доступных типов — вызови describe_task_types() без аргументов.",
        });
    }

    json!({
        "ok": true,
        "total": metas.len(),
        "task_types": metas,
        "hint": format!(
            "{TZ_HINT} external_apis.rate_limit_desc и constraints — основание для рекомендаций \
             по частоте запуска. config_fields описывает схему config_json конкретного задания \
             (см. list_scheduled_tasks)."
        ),
    })
}
