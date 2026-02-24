//! Исполнитель инструментов (tool calls) для LLM.
//!
//! Содержит:
//! - определения инструментов для передачи LLM (`metadata_tool_definitions`)
//! - диспетчер выполнения (`execute_tool_call`)

use super::knowledge_base::KNOWLEDGE_BASE;
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
        ToolDefinition {
            name: "search_knowledge".into(),
            description: "Поиск справочных материалов по тегам (база знаний Obsidian). \
                          Используй когда нужно понять бизнес-термин, метрику или получить \
                          контекст о сущности домена. \
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
                let results = KNOWLEDGE_BASE.search_by_tags(&tag_refs);
                let items: Vec<serde_json::Value> = results
                    .iter()
                    .map(|doc| serde_json::json!({
                        "id":      doc.id,
                        "title":   doc.title,
                        "tags":    doc.tags,
                        "related": doc.related,
                    }))
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
            match KNOWLEDGE_BASE.get(&id) {
                Some(doc) => serde_json::json!({
                    "id":      doc.id,
                    "title":   doc.title,
                    "tags":    doc.tags,
                    "related": doc.related,
                    "content": doc.content,
                }),
                None => {
                    let available: Vec<&str> = KNOWLEDGE_BASE
                        .all_docs()
                        .iter()
                        .map(|d| d.id.as_str())
                        .collect();
                    serde_json::json!({
                        "error": format!("Document '{}' not found.", id),
                        "available_ids": available,
                    })
                }
            }
        }

        unknown => serde_json::json!({
            "error": format!(
                "Unknown tool: '{}'. Available tools: list_entities, get_entity_schema, get_join_hint, search_knowledge, get_knowledge.",
                unknown
            )
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
