//! Исполнитель инструментов (tool calls) для LLM.
//!
//! Содержит:
//! - определения общих metadata-инструментов для передачи LLM
//! - диспетчер выполнения (`execute_tool_call`)

use super::admin_tools::execute_admin_tool;
use super::chart_tools::{execute_chart_tool, CHART_TOOL_NAMES};
use super::data_tools::{execute_data_tool, DATA_TOOL_NAMES};
use super::kb_admin_tools::execute_kb_admin_tool;
use super::mail_tools::{execute_mail_tool, MAIL_TOOL_NAMES};
use super::metadata_registry::METADATA_REGISTRY;
use super::plugin_tools::{execute_plugin_tool, PLUGIN_TOOL_NAMES};
use super::schedule_tools::{execute_schedule_tool, SCHEDULE_TOOL_NAMES};
use super::table_tools::{execute_build_table, execute_table_tool, TABLE_TOOL_NAMES};
use super::types::{ToolCall, ToolDefinition};
use contracts::domain::a017_llm_agent::aggregate::AgentType;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Идемпотентные инструменты «знания о системе»: результат зависит только от
/// (agent_type, name, arguments) — не от чата/состояния. Кэшируем их, чтобы LLM
/// не переоткрывал схему/каталог на каждом ходу диалога (лишние round-trip'ы и токены).
const CACHEABLE_TOOLS: &[&str] = &[
    "get_architecture_overview",
    "get_chart_of_accounts",
    "get_entity_schema",
    "list_entities",
    "get_join_hint",
    "list_data_sources",
];

/// Процесс-кэш результатов идемпотентных инструментов. Инвалидация — рестартом
/// процесса (метаданные/схема статичны в рамках запуска).
static METADATA_TOOL_CACHE: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn tool_result_ok(result: &serde_json::Value) -> bool {
    result
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or_else(|| result.get("error").is_none())
}

#[cfg(test)]
mod result_status_tests {
    use super::tool_result_ok;
    use serde_json::json;

    #[test]
    fn explicit_ok_false_wins_even_without_error_field() {
        assert!(!tool_result_ok(
            &json!({ "ok": false, "failures": ["render"] })
        ));
        assert!(tool_result_ok(
            &json!({ "ok": true, "error": "diagnostic only" })
        ));
        assert!(!tool_result_ok(&json!({ "error": "failed" })));
        assert!(tool_result_ok(&json!({ "result": 1 })));
    }
}

/// Ключ кэша. Включает agent_type, т.к. часть инструментов отдаёт ошибку доступа
/// для отдельных ролей (напр. list_entities для SystemAdmin).
fn cache_key(agent_type: &AgentType, name: &str, arguments: &str) -> String {
    format!("{}\u{0}{}\u{0}{}", agent_type.as_str(), name, arguments)
}

// ─── Определения инструментов ────────────────────────────────────────────────

/// Общие инструменты для всех агентов (схемы, KB, DataView).
pub(crate) fn shared_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_architecture_overview".into(),
            description: "Получить КАРТУ всей системы за один вызов: список сущностей \
                          (index, table, name, tags) и их связи (related). Используй В ПЕРВУЮ \
                          ОЧЕРЕДЬ, чтобы понять структуру домена, вместо множества list_entities. \
                          Затем углубляйся через get_entity_schema(index)."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Необязательный фильтр по тегу-категории.",
                        "enum": ["wb", "ozon", "ym", "ref", "llm", "promotion", "advertising",
                                 "bi", "dashboard", "projection", "gl", "accounting", "sales", "orders", "1c"]
                    }
                }
            }),
        },
        ToolDefinition {
            name: "get_chart_of_accounts".into(),
            description: "Получить план счетов General Ledger: код, имя, тип счёта \
                          (актив/пассив), нормальное сальдо, иерархию (parent_code), раздел \
                          отчётности и описание. Используй для понимания учётной модели: какие \
                          счета дебетуются/кредитуются, как устроены взаиморасчёты с маркетплейсом \
                          (7609/76YA/76YB), выручка (9001), себестоимость (9002). \
                          Виды оборотов между счетами — в list_gl_turnovers."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "get_entity_schema".into(),
            description: "Получить детальную схему таблицы: поля, SQL-типы, описания, \
                          внешние ключи (FK). Используй ПЕРЕД написанием SQL-запроса. \
                          Примеры entity_index: 'a004' (номенклатура), 'a012' (продажи WB), \
                          'a013' (заказы YM), 'a006' (подключения МП), 'a002' (организации)."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_index": {
                        "type": "string",
                        "description": "Короткий индекс сущности из list_entities, например 'a012', 'a004', 'a006'. Это НЕ schema_id из list_data_sources (напр. ds03_p904_sales) и не имя таблицы — для запроса данных по безопасной схеме используй query_data_schema."
                    }
                },
                "required": ["entity_index"]
            }),
        },
        ToolDefinition {
            name: "get_knowledge".into(),
            description: "Получить полное содержимое справочного материала по id. \
                          id берётся из результата search_knowledge."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Идентификатор документа из search_knowledge, например 'wb-promotions'."
                    }
                },
                "required": ["id"]
            }),
        },
    ]
}

/// Инструменты бизнес-аналитика (данные маркетплейсов, SQL, BI).
pub(crate) fn analyst_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_entities".into(),
            description: "Получить список таблиц базы данных с кратким описанием. \
                          ВСЕГДА передавай category — не запрашивай все таблицы без фильтра. \
                          Категории: wb=Wildberries (продажи), \
                          ozon=OZON, ym=Яндекс.Маркет, ref=справочники (организации, номенклатура), \
                          llm=чаты/агенты, bi=BI-индикаторы и дашборды, dashboard=то же что bi. \
                          Если уже знаешь entity_index — сразу вызывай get_entity_schema."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Необязательный фильтр по категории данных.",
                        "enum": ["wb", "ozon", "ym", "ref", "llm", "promotion", "bi", "dashboard", "gl", "accounting"]
                    }
                }
            }),
        },
        ToolDefinition {
            name: "get_join_hint".into(),
            description: "Получить подсказку как соединить (JOIN) две таблицы. \
                          Возвращает готовый SQL JOIN и имена FK-колонок."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "from_entity": {
                        "type": "string",
                        "description": "Индекс таблицы FROM, например 'a012'."
                    },
                    "to_entity": {
                        "type": "string",
                        "description": "Индекс таблицы для JOIN, например 'a006'."
                    }
                },
                "required": ["from_entity", "to_entity"]
            }),
        },
        ToolDefinition {
            name: "search_knowledge".into(),
            description: "Поиск справочных материалов по тегам. Obsidian-часть содержит \
                          бизнес-знания организации; embedded-документы содержат технический \
                          контекст приложения. Используй когда нужно понять бизнес-термин, \
                          метрику или получить контекст о сущности домена. \
                          Теги совпадают с entity_index (a020, a012, ...) и ключевыми словами: \
                          'drr', 'cpm', 'roas', 'акции', 'скидки', 'комиссии', 'wildberries', 'ozon'. \
                          Возвращает список документов (id, title) — затем вызывай get_knowledge(id)."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Список тегов для поиска. OR-семантика: совпадение хотя бы по одному. \
                                        Примеры: ['a020'] для акций WB, ['drr'] для метрики ДРР, \
                                        ['wildberries', 'комиссии'] для комиссий WB."
                    }
                },
                "required": ["tags"]
            }),
        },
        ToolDefinition {
            name: "create_drilldown_report".into(),
            description: "Создать drilldown-отчёт и сохранить его в системе. \
                          Инструмент записывает сессию в базу и возвращает artifact_id — \
                          пользователь увидит карточку с кнопкой открытия отчёта прямо в чате. \
                          Используй list_data_sources(kind=\"dataview\") чтобы узнать доступные view_id, metric_id и group_by. \
                          Обязательно уточни у пользователя период (date_from, date_to) если он не указан."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "view_id": { "type": "string", "description": "ID DataView, например 'dv001_revenue'." },
                    "group_by": { "type": "string", "description": "Измерение для детализации." },
                    "metric_id": { "type": "string", "description": "Метрика: 'revenue', 'cost', 'commission', 'expenses', 'profit', 'profit_d'." },
                    "date_from": { "type": "string", "description": "Начало периода, YYYY-MM-DD." },
                    "date_to": { "type": "string", "description": "Конец периода, YYYY-MM-DD." },
                    "description": { "type": "string", "description": "Человекочитаемое название отчёта." },
                    "period2_from": { "type": "string", "description": "Начало периода сравнения (опционально)." },
                    "period2_to": { "type": "string", "description": "Конец периода сравнения (опционально)." },
                    "params": {
                        "type": "object",
                        "description": "Дополнительные параметры DataView, например {\"layer\":\"fact\",\"turnover_code\":\"mp_commission\"} для dv004."
                    },
                    "connection_mp_refs": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UUID кабинетов МП для фильтрации (опционально, пустой = все)."
                    }
                },
                "required": ["view_id", "group_by", "metric_id", "date_from", "date_to", "description"]
            }),
        },
        ToolDefinition {
            name: "list_gl_turnovers".into(),
            description: "Получить список видов оборотов General Ledger \
                          (turnover_code, name, description, счета Дт/Кт, формулы). \
                          Используй для понимания структуры учёта: какие операции фиксируются \
                          в sys_general_ledger, какой turnover_code использовать в WHERE-условии, \
                          какой счёт дебетуется/кредитуется при продаже/возврате/комиссии."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "report_group": {
                        "type": "string",
                        "description": "Фильтр по группе отчёта: revenue, returns, commission, \
                                        acquiring, logistics, storage, penalty, advertising, \
                                        cost, quantity, ratio, adjustment, other",
                        "enum": [
                            "revenue", "returns", "payout", "commission", "acquiring",
                            "logistics", "storage", "penalty", "advertising",
                            "cost", "quantity", "ratio", "adjustment", "other"
                        ]
                    }
                }
            }),
        },
    ]
}

// ─── Диспетчер ───────────────────────────────────────────────────────────────

/// Выполнить tool call и вернуть результат в виде JSON-строки.
///
/// `chat_id` и `agent_id` нужны для создания артефактов (a019_llm_artifact).
/// `agent_type` определяет допустимый набор инструментов.
/// Вызывается в цикле `send_message`, когда LLM возвращает `tool_calls`.
pub async fn execute_tool_call(
    call: &ToolCall,
    chat_id: &str,
    agent_id: &str,
    agent_type: &AgentType,
    active_tools: &HashSet<String>,
) -> String {
    // Авторизация: исполняем только инструменты активного набора (core ∪ активные навыки).
    // Единый источник истины вместо разрозненных проверок по роли агента.
    if !active_tools.contains(call.name.as_str()) {
        let result = serde_json::json!({
            "error": format!(
                "Инструмент '{}' не активен в текущем наборе. Вызови list_skills() и \
                 use_skill(\"<id>\"), чтобы активировать нужный навык.",
                call.name
            ),
            "_tool": call.name,
            "_ok": false,
        });
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Кэш идемпотентных «системных» инструментов — обслуживаем повтор без вычисления.
    let cacheable = CACHEABLE_TOOLS.contains(&call.name.as_str());
    if cacheable {
        let key = cache_key(agent_type, &call.name, &call.arguments);
        if let Ok(cache) = METADATA_TOOL_CACHE.lock() {
            if let Some(hit) = cache.get(&key) {
                return hit.clone();
            }
        }
    }

    if matches!(
        call.name.as_str(),
        "list_kb_documents"
            | "get_kb_document"
            | "create_kb_edit"
            | "update_kb_edit_articles"
            | "list_open_kb_edits"
            | "write_kb_document"
    ) {
        let result = execute_kb_admin_tool(&call.name, &call.arguments, agent_id).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    if DATA_TOOL_NAMES.contains(&call.name.as_str()) {
        let result =
            execute_data_tool(&call.name, &call.arguments, agent_type, chat_id, agent_id).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        let output = serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
        if cacheable {
            if let Ok(mut cache) = METADATA_TOOL_CACHE.lock() {
                cache.insert(
                    cache_key(agent_type, &call.name, &call.arguments),
                    output.clone(),
                );
            }
        }
        return output;
    }

    // build_chart — высокоуровневый сборщик: async + контекст чата (отдельная ветка,
    // т.к. он выполняет SQL и сохраняет плагин, в отличие от заготовок-инструментов).
    if call.name == "build_chart" {
        let result =
            super::chart_tools::execute_build_chart(&call.arguments, chat_id, agent_id).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    if call.name == "build_table" {
        let result = execute_build_table(&call.arguments, chat_id, agent_id).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Chart builder tools — dispatch to chart_tools module (заготовки, без БД)
    if CHART_TOOL_NAMES.contains(&call.name.as_str()) {
        let result = execute_chart_tool(&call.name, &call.arguments);
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Table builder tools — dispatch to table_tools module (заготовки, без БД)
    if TABLE_TOOL_NAMES.contains(&call.name.as_str()) {
        let result = execute_table_tool(&call.name, &call.arguments);
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Mail tools (IMAP/SMTP) — dispatch to mail_tools module
    if MAIL_TOOL_NAMES.contains(&call.name.as_str()) {
        let result = execute_mail_tool(&call.name, &call.arguments).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Scheduler tools (регламентные задания) — dispatch to schedule_tools module
    if SCHEDULE_TOOL_NAMES.contains(&call.name.as_str()) {
        let result = execute_schedule_tool(&call.name, &call.arguments).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Plugin developer tools — dispatch to plugin_tools module
    if PLUGIN_TOOL_NAMES.contains(&call.name.as_str()) {
        let result = execute_plugin_tool(&call.name, &call.arguments, chat_id, agent_id).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    // Admin-only tools — dispatch to admin_tools module
    if matches!(
        call.name.as_str(),
        "check_system_health"
            | "get_performance_stats"
            | "list_background_jobs"
            | "get_data_integrity_report"
    ) {
        let result = execute_admin_tool(&call.name, &call.arguments).await;
        let is_ok = tool_result_ok(&result);
        let mut result = result;
        if let serde_json::Value::Object(ref mut map) = result {
            map.insert(
                "_tool".to_string(),
                serde_json::Value::String(call.name.clone()),
            );
            map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
        }
        return serde_json::to_string_pretty(&result)
            .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));
    }

    let result = match call.name.as_str() {
        "get_architecture_overview" => {
            let category = parse_string_arg(&call.arguments, "category");
            METADATA_REGISTRY.architecture_overview(category.as_deref())
        }

        "get_chart_of_accounts" => {
            let accounts = crate::shared::analytics::account_registry::ACCOUNT_REGISTRY;
            serde_json::json!({
                "accounts": accounts,
                "count": accounts.len(),
                "hint": "План счетов GL. parent_code задаёт иерархию (группа → субсчёт). \
                         Проводки хранятся в sys_general_ledger (поля debit_account/credit_account). \
                         Какие обороты задействуют счета — см. list_gl_turnovers."
            })
        }

        "list_entities" => {
            let category = parse_string_arg(&call.arguments, "category");
            METADATA_REGISTRY.list_entities(category.as_deref())
        }

        "list_skills" => super::skills::list_skills_result(agent_type),

        "use_skill" => super::skills::use_skill_result(&call.arguments, agent_type),

        "get_entity_schema" => {
            let index = parse_string_arg(&call.arguments, "entity_index").unwrap_or_default();
            tracing::info!("[get_entity_schema] called with entity_index='{}'", index);
            let result = METADATA_REGISTRY.get_entity_schema(&index);
            let fields_count = result
                .get("fields")
                .and_then(|f| f.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            tracing::info!(
                "[get_entity_schema] entity='{}' fields_count={} has_error={}",
                index,
                fields_count,
                result.get("error").is_some()
            );
            result
        }

        "get_join_hint" => {
            let from = parse_string_arg(&call.arguments, "from_entity").unwrap_or_default();
            let to = parse_string_arg(&call.arguments, "to_entity").unwrap_or_default();
            METADATA_REGISTRY.get_join_hint(&from, &to)
        }

        "search_knowledge" => {
            let tags_value = serde_json::from_str::<serde_json::Value>(&call.arguments)
                .ok()
                .and_then(|v| v.get("tags").cloned());

            let tags: Vec<String> = match tags_value {
                Some(serde_json::Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                _ => vec![],
            };

            if tags.is_empty() {
                serde_json::json!({
                    "error": "Parameter 'tags' is required and must be a non-empty array of strings."
                })
            } else {
                let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
                let kb = super::knowledge_base::kb_read();
                let results = kb.search_by_tags(&tag_refs);
                let items: Vec<serde_json::Value> = results
                    .iter()
                    .map(|doc| {
                        serde_json::json!({
                            "id":      doc.id,
                            "title":   doc.title,
                            "tags":    doc.tags,
                            "related": doc.related,
                            "source_path": doc.source_path,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "results": items,
                    "total": items.len(),
                    "hint": "Используй get_knowledge(id) чтобы получить полное содержимое документа."
                })
            }
        }

        "get_knowledge" => {
            let id = parse_string_arg(&call.arguments, "id").unwrap_or_default();
            let kb = super::knowledge_base::kb_read();
            match kb.get(&id) {
                Some(doc) => serde_json::json!({
                    "id":      doc.id,
                    "title":   doc.title,
                    "tags":    doc.tags,
                    "related": doc.related,
                    "source_path": doc.source_path,
                    "content": doc.content,
                }),
                None => {
                    let available: Vec<&str> =
                        kb.all_docs().iter().map(|d| d.id.as_str()).collect();
                    serde_json::json!({
                        "error": format!("Document '{}' not found.", id),
                        "available_ids": available,
                    })
                }
            }
        }

        "create_drilldown_report" => {
            create_drilldown_report_tool(&call.arguments, chat_id, agent_id).await
        }

        "list_gl_turnovers" => {
            let args =
                serde_json::from_str::<serde_json::Value>(&call.arguments).unwrap_or_default();
            let report_group = args.get("report_group").and_then(|v| v.as_str());
            let items: Vec<_> = crate::general_ledger::turnover_registry::TURNOVER_CLASSES
                .iter()
                .filter(|t| report_group.map_or(true, |g| t.report_group.as_str() == g))
                .map(|t| {
                    serde_json::json!({
                        "code": t.code,
                        "name": t.name,
                        "description": t.description,
                        "llm_description": t.llm_description,
                        "debit_account": t.debit_account,
                        "credit_account": t.credit_account,
                        "report_group": t.report_group.as_str(),
                        "generates_journal_entry": t.generates_journal_entry,
                        "formula_hint": t.formula_hint,
                    })
                })
                .collect();
            let count = items.len();
            serde_json::json!({
                "turnovers": items,
                "count": count,
                "hint": "Используй turnover_code в WHERE sys_general_ledger.turnover_code = '...' \
                         для фильтрации проводок нужного типа."
            })
        }

        unknown => serde_json::json!({
            "error": format!(
                "Unknown tool: '{}'. Available tools depend on agent type. \
                 BusinessAnalyst: list_entities, get_entity_schema, get_join_hint, \
                 search_knowledge, get_knowledge, list_data_sources, execute_query, \
                 create_drilldown_report, list_gl_turnovers. \
                 SystemAdmin: get_entity_schema, get_knowledge, \
                 check_system_health, get_performance_stats, list_background_jobs, \
                 get_data_integrity_report.",
                unknown
            )
        }),
    };

    // Добавляем отладочные метаданные: _tool и _ok
    let is_ok = tool_result_ok(&result);
    let mut result = result;
    if let serde_json::Value::Object(ref mut map) = result {
        map.insert(
            "_tool".to_string(),
            serde_json::Value::String(call.name.clone()),
        );
        map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
    }

    let output = serde_json::to_string_pretty(&result)
        .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e));

    // Сохранить в кэш идемпотентных инструментов.
    if cacheable {
        if let Ok(mut cache) = METADATA_TOOL_CACHE.lock() {
            cache.insert(
                cache_key(agent_type, &call.name, &call.arguments),
                output.clone(),
            );
        }
    }

    output
}

// ─── create_drilldown_report implementation ───────────────────────────────────

async fn create_drilldown_report_tool(
    arguments_json: &str,
    chat_id: &str,
    agent_id: &str,
) -> serde_json::Value {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use uuid::Uuid;

    // Parse arguments
    let args: serde_json::Value = match serde_json::from_str(arguments_json) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("Failed to parse tool arguments: {}", e)
            });
        }
    };

    let view_id = match args.get("view_id").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return serde_json::json!({ "error": "Missing required parameter: view_id" }),
    };
    let group_by = match args.get("group_by").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return serde_json::json!({ "error": "Missing required parameter: group_by" }),
    };
    let metric_id = match args.get("metric_id").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return serde_json::json!({ "error": "Missing required parameter: metric_id" }),
    };
    let date_from = match args.get("date_from").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return serde_json::json!({ "error": "Missing required parameter: date_from" }),
    };
    let date_to = match args.get("date_to").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return serde_json::json!({ "error": "Missing required parameter: date_to" }),
    };
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("Drilldown отчёт")
        .to_string();

    let period2_from = args
        .get("period2_from")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let period2_to = args
        .get("period2_to")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let connection_mp_refs: Vec<String> = args
        .get("connection_mp_refs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let extra_params = args
        .get("params")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    // Validate DataView exists
    let registry = crate::data_view::DataViewRegistry::new();
    if !registry.has_view(&view_id) {
        return serde_json::json!({
            "error": format!("DataView '{}' not found. Use list_data_sources(kind=\"dataview\") to see available views.", view_id)
        });
    }

    let db = crate::shared::data::db::get_connection();

    // 1. Create sys_drilldown session
    let session_id = Uuid::new_v4().to_string();
    let params_json = serde_json::json!({
        "view_id": view_id,
        "metric_id": null,
        "metric_ids": [metric_id.clone()],
        "group_by": group_by,
        "group_by_label": "",
        "date_from": date_from,
        "date_to": date_to,
        "period2_from": period2_from,
        "period2_to": period2_to,
        "connection_mp_refs": connection_mp_refs,
        "params": extra_params
    });

    if let Err(e) = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO sys_drilldown (id, view_id, indicator_id, indicator_name, params_json) \
             VALUES (?, ?, '', ?, ?)",
            [
                session_id.clone().into(),
                view_id.clone().into(),
                description.clone().into(),
                params_json.to_string().into(),
            ],
        ))
        .await
    {
        tracing::error!("Failed to create sys_drilldown session: {}", e);
        return serde_json::json!({
            "error": format!("Failed to create drilldown session: {}", e)
        });
    }

    // 2. Create a019_llm_artifact
    let artifact_query_params = serde_json::json!({
        "session_id": session_id,
        "view_id": view_id,
        "group_by": group_by,
        "metric_id": metric_id,
        "date_from": date_from,
        "date_to": date_to,
        "params": extra_params,
    });

    let artifact_dto = crate::domain::a019_llm_artifact::service::LlmArtifactDto {
        id: None,
        code: Some(format!("DRILLDOWN-{}", &session_id[..8].to_uppercase())),
        description: description.clone(),
        comment: Some(format!(
            "Отчёт: {} по {}, период {} — {}",
            metric_id, group_by, date_from, date_to
        )),
        chat_id: chat_id.to_string(),
        agent_id: agent_id.to_string(),
        artifact_type: Some("drilldown_report".to_string()),
        sql_query: String::new(),
        query_params: Some(artifact_query_params.to_string()),
        visualization_config: None,
    };

    match crate::domain::a019_llm_artifact::service::create(artifact_dto).await {
        Ok(artifact_uuid) => {
            tracing::info!(
                "Created drilldown artifact {} for session {}",
                artifact_uuid,
                session_id
            );
            serde_json::json!({
                "success": true,
                "artifact_id": artifact_uuid.to_string(),
                "session_id": session_id,
                "description": description,
                "hint": "Артефакт создан. Пользователь увидит карточку с кнопкой открытия отчёта в чате."
            })
        }
        Err(e) => {
            tracing::error!("Failed to create drilldown artifact: {}", e);
            // Session was created, return partial success so user can still navigate
            serde_json::json!({
                "success": false,
                "session_id": session_id,
                "error": format!("Session created but artifact save failed: {}", e)
            })
        }
    }
}

// ─── Вспомогательные ─────────────────────────────────────────────────────────

/// Извлечь строковый аргумент из JSON-строки аргументов tool call.
fn parse_string_arg(arguments_json: &str, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(arguments_json)
        .ok()
        .and_then(|v| v.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
}
