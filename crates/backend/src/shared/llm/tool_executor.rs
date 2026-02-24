//! Исполнитель инструментов (tool calls) для LLM.
//!
//! Содержит:
//! - определения инструментов для передачи LLM (`metadata_tool_definitions`)
//! - диспетчер выполнения (`execute_tool_call`)

use super::metadata_registry::METADATA_REGISTRY;
use super::types::{ToolCall, ToolDefinition};

// ─── Определения инструментов ────────────────────────────────────────────────

/// Вернуть определения инструментов для работы с метаданными схемы.
/// Передаётся в `chat_completion_with_tools` при каждом запросе к LLM.
pub fn metadata_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_entities".into(),
            description: "Получить список таблиц базы данных с кратким описанием. \
                          ВСЕГДА передавай category — не запрашивай все таблицы без фильтра. \
                          Категории: wb=Wildberries (продажи), \
                          ozon=OZON, ym=Яндекс.Маркет, ref=справочники (организации, номенклатура), \
                          llm=чаты/агенты. \
                          Если уже знаешь entity_index — сразу вызывай get_entity_schema."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Необязательный фильтр по категории данных.",
                        "enum": ["wb", "ozon", "ym", "ref", "llm", "promotion"]
                    }
                }
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
                        "description": "Индекс сущности из list_entities, например: 'a012', 'a004', 'a006'."
                    }
                },
                "required": ["entity_index"]
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
    ]
}

// ─── Диспетчер ───────────────────────────────────────────────────────────────

/// Выполнить tool call и вернуть результат в виде JSON-строки.
///
/// Вызывается в цикле `send_message`, когда LLM возвращает `tool_calls`.
pub fn execute_tool_call(call: &ToolCall) -> String {
    let result = match call.name.as_str() {
        "list_entities" => {
            let category = parse_string_arg(&call.arguments, "category");
            METADATA_REGISTRY.list_entities(category.as_deref())
        }

        "get_entity_schema" => {
            let index = parse_string_arg(&call.arguments, "entity_index").unwrap_or_default();
            METADATA_REGISTRY.get_entity_schema(&index)
        }

        "get_join_hint" => {
            let from = parse_string_arg(&call.arguments, "from_entity").unwrap_or_default();
            let to = parse_string_arg(&call.arguments, "to_entity").unwrap_or_default();
            METADATA_REGISTRY.get_join_hint(&from, &to)
        }

        unknown => serde_json::json!({
            "error": format!("Unknown tool: '{}'. Available tools: list_entities, get_entity_schema, get_join_hint.", unknown)
        }),
    };

    serde_json::to_string_pretty(&result)
        .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e))
}

// ─── Вспомогательные ─────────────────────────────────────────────────────────

/// Извлечь строковый аргумент из JSON-строки аргументов tool call.
fn parse_string_arg(arguments_json: &str, key: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(arguments_json)
        .ok()
        .and_then(|v| v.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
}
