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
        ToolDefinition {
            name: "list_data_views".into(),
            description: "Получить список доступных DataView — именованных бизнес-вычислений \
                          (семантический слой над таблицами БД). \
                          Каждый DataView описывает: метрики (metric_id) и измерения (group_by) \
                          для drill-down детализации. \
                          Используй для выбора view_id и metric_id при создании BI-индикаторов \
                          или при вопросах о доступных аналитических срезах."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDefinition {
            name: "execute_query".into(),
            description: "Выполнить SQL SELECT-запрос к базе данных и получить результат. \
                          ТОЛЬКО SELECT (WITH ... SELECT тоже разрешён). INSERT/UPDATE/DELETE — запрещены. \
                          Результат возвращается как массив rows + сохраняется как артефакт в чате. \
                          Используй для поиска UUID в справочниках перед созданием drilldown-отчёта. \
                          Обязательно вызывай get_entity_schema перед написанием SQL — \
                          имена таблиц и колонок должны точно совпадать со схемой. \
                          Примеры таблиц справочников: a006_connection_mp (кабинеты МП), \
                          a002_organization (организации), a004_nomenclature (номенклатура), \
                          a005_marketplace (маркетплейсы)."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "sql": {
                        "type": "string",
                        "description": "SQL SELECT-запрос. Автоматически добавляется LIMIT 50 если не указан. \
                                        Максимум 200 строк. Используй конкретные WHERE-условия для фильтрации."
                    },
                    "description": {
                        "type": "string",
                        "description": "Описание что ищем — сохраняется как название артефакта, например \
                                        'Кабинеты Wildberries' или 'Проверка наличия артикула ABC-123'."
                    }
                },
                "required": ["sql", "description"]
            }),
        },
        ToolDefinition {
            name: "create_drilldown_report".into(),
            description: "Создать drilldown-отчёт и сохранить его в системе. \
                          Инструмент записывает сессию в базу и возвращает artifact_id — \
                          пользователь увидит карточку с кнопкой открытия отчёта прямо в чате. \
                          Используй list_data_views чтобы узнать доступные view_id, metric_id и group_by. \
                          Обязательно уточни у пользователя период (date_from, date_to) если он не указан."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "view_id": {
                        "type": "string",
                        "description": "ID DataView, например 'dv001_revenue'."
                    },
                    "group_by": {
                        "type": "string",
                        "description": "Измерение для детализации: 'marketplace', 'date', 'article', \
                                        'connection_mp_ref', 'nomenclature_ref', 'dim1'..'dim6'."
                    },
                    "metric_id": {
                        "type": "string",
                        "description": "Метрика: 'revenue', 'cost', 'commission', 'expenses', 'profit', 'profit_d'."
                    },
                    "date_from": {
                        "type": "string",
                        "description": "Начало периода, YYYY-MM-DD."
                    },
                    "date_to": {
                        "type": "string",
                        "description": "Конец периода, YYYY-MM-DD."
                    },
                    "description": {
                        "type": "string",
                        "description": "Человекочитаемое название отчёта, например 'Выручка по маркетплейсам, январь 2026'."
                    },
                    "period2_from": {
                        "type": "string",
                        "description": "Начало периода сравнения (опционально). Если не задан — автоматически -1 месяц."
                    },
                    "period2_to": {
                        "type": "string",
                        "description": "Конец периода сравнения (опционально)."
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
/// Вызывается в цикле `send_message`, когда LLM возвращает `tool_calls`.
pub async fn execute_tool_call(call: &ToolCall, chat_id: &str, agent_id: &str) -> String {
    let result = match call.name.as_str() {
        "list_entities" => {
            let category = parse_string_arg(&call.arguments, "category");
            METADATA_REGISTRY.list_entities(category.as_deref())
        }

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
                let results = KNOWLEDGE_BASE.search_by_tags(&tag_refs);
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
            match KNOWLEDGE_BASE.get(&id) {
                Some(doc) => serde_json::json!({
                    "id":      doc.id,
                    "title":   doc.title,
                    "tags":    doc.tags,
                    "related": doc.related,
                    "source_path": doc.source_path,
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

        "list_data_views" => {
            let registry = crate::data_view::DataViewRegistry::new();
            let views: Vec<serde_json::Value> = registry
                .list_meta()
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "id":          m.id,
                        "name":        m.name,
                        "category":    m.category,
                        "description": m.ai_description,
                        "data_sources": m.data_sources,
                        "metrics":     m.available_resources.iter().map(|r| serde_json::json!({
                            "id":          r.id,
                            "label":       r.label,
                            "description": r.description,
                            "unit":        r.unit,
                        })).collect::<Vec<_>>(),
                        "dimensions":  m.available_dimensions.iter().map(|d| serde_json::json!({
                            "id":    d.id,
                            "label": d.label,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            let total = views.len();
            serde_json::json!({
                "data_views": views,
                "total": total,
                "hint": "Используй view_id и metric_id при создании BI-индикатора (a024). \
                         Для drill-down детализации используй id из dimensions в качестве group_by."
            })
        }

        "execute_query" => execute_query_tool(&call.arguments, chat_id, agent_id).await,

        "create_drilldown_report" => {
            create_drilldown_report_tool(&call.arguments, chat_id, agent_id).await
        }

        "list_gl_turnovers" => {
            let args = serde_json::from_str::<serde_json::Value>(&call.arguments).unwrap_or_default();
            let report_group = args.get("report_group").and_then(|v| v.as_str());
            let items: Vec<_> = crate::general_ledger::turnover_registry::TURNOVER_CLASSES
                .iter()
                .filter(|t| report_group.map_or(true, |g| t.report_group.as_str() == g))
                .map(|t| serde_json::json!({
                    "code": t.code,
                    "name": t.name,
                    "description": t.description,
                    "llm_description": t.llm_description,
                    "debit_account": t.debit_account,
                    "credit_account": t.credit_account,
                    "report_group": t.report_group.as_str(),
                    "generates_journal_entry": t.generates_journal_entry,
                    "formula_hint": t.formula_hint,
                }))
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
                "Unknown tool: '{}'. Available tools: list_entities, get_entity_schema, \
                 get_join_hint, search_knowledge, get_knowledge, list_data_views, \
                 execute_query, create_drilldown_report, list_gl_turnovers.",
                unknown
            )
        }),
    };

    // Добавляем отладочные метаданные: _tool и _ok
    let is_ok = result.get("error").is_none();
    let mut result = result;
    if let serde_json::Value::Object(ref mut map) = result {
        map.insert(
            "_tool".to_string(),
            serde_json::Value::String(call.name.clone()),
        );
        map.insert("_ok".to_string(), serde_json::Value::Bool(is_ok));
    }

    serde_json::to_string_pretty(&result)
        .unwrap_or_else(|e| format!("{{\"error\": \"Serialization error: {}\"}}", e))
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

    // Validate DataView exists
    let registry = crate::data_view::DataViewRegistry::new();
    if !registry.has_view(&view_id) {
        return serde_json::json!({
            "error": format!("DataView '{}' not found. Use list_data_views to see available views.", view_id)
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
        "params": {}
    });

    let params_json_str = params_json.to_string().replace('\'', "''");

    let insert_session_sql = format!(
        "INSERT INTO sys_drilldown (id, view_id, indicator_id, indicator_name, params_json) \
         VALUES ('{}', '{}', '', '{}', '{}')",
        session_id,
        view_id.replace('\'', "''"),
        description.replace('\'', "''"),
        params_json_str,
    );

    if let Err(e) = db
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            insert_session_sql,
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

// ─── execute_query implementation ────────────────────────────────────────────

/// Максимальное число строк в результате execute_query.
const QUERY_MAX_ROWS: usize = 200;
/// Лимит по умолчанию если LLM не указал LIMIT.
const QUERY_DEFAULT_LIMIT: usize = 50;

async fn execute_query_tool(
    arguments_json: &str,
    chat_id: &str,
    agent_id: &str,
) -> serde_json::Value {
    use sea_orm::{DatabaseBackend, FromQueryResult, Statement};

    // 1. Парсим аргументы
    let args: serde_json::Value = match serde_json::from_str(arguments_json) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("Failed to parse tool arguments: {}", e)
            })
        }
    };

    let raw_sql = match args.get("sql").and_then(|v| v.as_str()) {
        Some(s) => s.trim().to_string(),
        None => return serde_json::json!({ "error": "Missing required parameter: sql" }),
    };
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("SQL-запрос")
        .to_string();

    // 2. Валидация: только SELECT
    let upper = raw_sql.to_uppercase();
    let trimmed_upper = upper.trim_start();
    let is_select = trimmed_upper.starts_with("SELECT") || trimmed_upper.starts_with("WITH");
    if !is_select {
        return serde_json::json!({
            "error": "Only SELECT queries are allowed. INSERT/UPDATE/DELETE/DROP are forbidden."
        });
    }
    // Запрет модифицирующих операций даже внутри CTE
    for forbidden in &[
        "INSERT ", "UPDATE ", "DELETE ", "DROP ", "ATTACH ", "PRAGMA ",
    ] {
        if upper.contains(forbidden) {
            return serde_json::json!({
                "error": format!("Forbidden keyword '{}' found in query.", forbidden.trim())
            });
        }
    }

    // 3. Принудительный LIMIT
    let sql = enforce_limit(&raw_sql, QUERY_DEFAULT_LIMIT, QUERY_MAX_ROWS);

    // 4. Выполняем запрос — через FromQueryResult для serde_json::Value
    // (sea-orm with-json feature даёт Vec<JsonValue> с именами колонок)
    let db = crate::shared::data::db::get_connection();
    let stmt = Statement::from_string(DatabaseBackend::Sqlite, sql.clone());
    let json_rows_result = serde_json::Value::find_by_statement(stmt).all(db).await;

    let json_rows: Vec<serde_json::Value> = match json_rows_result {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("execute_query SQL error: {}", e);
            return serde_json::json!({
                "error": format!("SQL execution error: {}", e),
                "sql": sql,
            });
        }
    };

    let actual_count = json_rows.len();
    let truncated = actual_count >= QUERY_MAX_ROWS;

    // 6. Сохраняем как a019_llm_artifact (SqlQuery)
    let artifact_id_opt =
        save_query_artifact(&sql, &description, chat_id, agent_id, actual_count).await;

    let mut result = serde_json::json!({
        "rows": json_rows,
        "row_count": actual_count,
        "truncated": truncated,
        "sql": sql,
    });

    if let Some(ref id) = artifact_id_opt {
        result["artifact_id"] = serde_json::Value::String(id.clone());
        result["hint"] = serde_json::Value::String(
            "Запрос сохранён как артефакт. Если нашёл нужные UUID — \
             используй их в create_drilldown_report.connection_mp_refs."
                .to_string(),
        );
    }

    result
}

/// Добавить/скорректировать LIMIT в SQL-запросе.
fn enforce_limit(sql: &str, default_limit: usize, max_limit: usize) -> String {
    let upper = sql.to_uppercase();
    // Ищем LIMIT N (последнее вхождение)
    if let Some(pos) = upper.rfind("LIMIT") {
        // Парсим число после LIMIT
        let after = sql[pos + 5..].trim_start();
        let num_end = after
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after.len());
        if let Ok(n) = after[..num_end].parse::<usize>() {
            if n > max_limit {
                // Заменяем превышающий лимит
                let limit_start = pos;
                let limit_full_end = pos + 5 + (after.len() - after.trim_start().len()) + num_end;
                return format!("{} LIMIT {}", sql[..limit_start].trim_end(), max_limit)
                    + &sql[limit_full_end..];
            }
            // Лимит в пределах нормы — не меняем
            return sql.to_string();
        }
    }
    // LIMIT не найден — добавляем
    format!(
        "{} LIMIT {}",
        sql.trim_end().trim_end_matches(';'),
        default_limit
    )
}

/// Сохранить SQL-запрос как a019_llm_artifact.
async fn save_query_artifact(
    sql: &str,
    description: &str,
    chat_id: &str,
    agent_id: &str,
    row_count: usize,
) -> Option<String> {
    let dto = crate::domain::a019_llm_artifact::service::LlmArtifactDto {
        id: None,
        code: None,
        description: description.to_string(),
        comment: Some(format!("Возвращено строк: {}", row_count)),
        chat_id: chat_id.to_string(),
        agent_id: agent_id.to_string(),
        artifact_type: Some("sql_query".to_string()),
        sql_query: sql.to_string(),
        query_params: None,
        visualization_config: None,
    };
    match crate::domain::a019_llm_artifact::service::create(dto).await {
        Ok(uuid) => {
            tracing::info!("execute_query: saved artifact {}", uuid);
            Some(uuid.to_string())
        }
        Err(e) => {
            tracing::warn!("execute_query: failed to save artifact: {}", e);
            None
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
