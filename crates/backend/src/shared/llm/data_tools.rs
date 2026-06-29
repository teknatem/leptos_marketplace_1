use super::types::ToolDefinition;
use crate::shared::data_access::{
    execute_raw_query, list_sources, query_schema, run_data_view_drilldown, run_data_view_scalar,
    DataSourceKind, DataViewDrilldownRequest, DataViewScalarRequest, RawQueryRequest,
    SchemaQueryRequest, SqlAccessProfile,
};
use contracts::domain::a017_llm_agent::aggregate::AgentType;
use serde::Deserialize;
use serde_json::{json, Value};

pub const DATA_TOOL_NAMES: &[&str] = &[
    "list_data_sources",
    "query_data_schema",
    "run_data_view_scalar",
    "run_data_view_drilldown",
    "execute_query",
];

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_data_sources".into(),
            description: "Единый каталог аналитических источников: безопасные base-схемы, \
                          курируемые DataView и Raw SQL fallback. Вызывай первым, затем выбирай: \
                          DataView для официальной метрики/2 периодов, base для ad-hoc среза, \
                          execute_query только если первые два пути не подходят."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "kind": {
                        "type": "string",
                        "enum": ["base", "dataview", "raw"],
                        "description": "Необязательный фильтр вида источника."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "query_data_schema".into(),
            description: "Безопасно получить строки из base-схемы без написания SQL. \
                          Поля, группировки, метрики и фильтры проверяются по allowlist схемы; \
                          значения передаются bind-параметрами."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "schema_id": { "type": "string", "description": "ID из list_data_sources, например ds03_p904_sales или a006." },
                    "fields": { "type": "array", "items": { "type": "string" }, "description": "Обычные поля результата без агрегации." },
                    "group_by": { "type": "array", "items": { "type": "string" }, "description": "Измерения группировки." },
                    "metrics": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field_id": { "type": "string" },
                                "aggregate": { "type": "string", "enum": ["sum", "count", "avg", "min", "max"] }
                            },
                            "required": ["field_id", "aggregate"]
                        }
                    },
                    "filters": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field_id": { "type": "string" },
                                "operator": { "type": "string", "enum": ["eq", "not_eq", "lt", "lte", "gt", "gte", "between", "in", "not_in", "contains", "is_null", "is_not_null"] },
                                "value": {},
                                "values": { "type": "array", "items": {} },
                                "from": {},
                                "to": {}
                            },
                            "required": ["field_id", "operator"]
                        }
                    },
                    "sort": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field_id": { "type": "string" },
                                "direction": { "type": "string", "enum": ["asc", "desc"] }
                            },
                            "required": ["field_id", "direction"]
                        }
                    },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
                },
                "required": ["schema_id"]
            }),
        },
        data_view_definition("run_data_view_scalar", false),
        data_view_definition("run_data_view_drilldown", true),
        ToolDefinition {
            name: "execute_query".into(),
            description: "Raw SQL fallback: один SQLite SELECT/WITH с AST-проверкой, профилем \
                          доступа, bind-параметрами, таймаутом и лимитом. Не читает таблицы \
                          подключений с credentials. Используй только когда DataView и base-схемы \
                          не покрывают запрос."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "SELECT с ? placeholders для значений." },
                    "params": { "type": "array", "items": {}, "description": "Скалярные bind-параметры в порядке placeholders." },
                    "description": { "type": "string", "description": "Название сохраняемого SQL-артефакта." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
                },
                "required": ["sql", "description"]
            }),
        },
    ]
}

fn data_view_definition(name: &str, drilldown: bool) -> ToolDefinition {
    let mut properties = json!({
        "view_id": { "type": "string", "description": "DataView ID из list_data_sources." },
        "date_from": { "type": "string", "description": "YYYY-MM-DD" },
        "date_to": { "type": "string", "description": "YYYY-MM-DD" },
        "period2_from": { "type": "string", "description": "YYYY-MM-DD; указывать парой с period2_to." },
        "period2_to": { "type": "string", "description": "YYYY-MM-DD; указывать парой с period2_from." },
        "connection_mp_refs": { "type": "array", "items": { "type": "string" } },
        "params": { "type": "object", "additionalProperties": { "type": "string" } }
    });
    let required = if drilldown {
        properties["group_by"] = json!({ "type": "string" });
        properties["metric_ids"] = json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 1
        });
        json!(["view_id", "group_by", "metric_ids", "date_from", "date_to"])
    } else {
        properties["metric_id"] = json!({ "type": "string" });
        json!(["view_id", "metric_id", "date_from", "date_to"])
    };
    ToolDefinition {
        name: name.to_string(),
        description: if drilldown {
            "Получить строки курируемого DataView по одному измерению и одной или нескольким метрикам."
        } else {
            "Вычислить курируемую метрику DataView со сравнением периодов."
        }
        .to_string(),
        parameters: json!({
            "type": "object",
            "properties": properties,
            "required": required
        }),
    }
}

#[derive(Deserialize)]
struct ListSourcesArgs {
    #[serde(default)]
    kind: Option<DataSourceKind>,
}

#[derive(Deserialize)]
struct ExecuteQueryArgs {
    sql: String,
    description: String,
    #[serde(default)]
    params: Vec<Value>,
    #[serde(default)]
    limit: Option<usize>,
}

fn access_profile(agent_type: &AgentType) -> Result<SqlAccessProfile, String> {
    match agent_type {
        AgentType::BusinessAnalyst | AgentType::PluginAdmin => Ok(SqlAccessProfile::Analytics),
        AgentType::KbAdmin => Ok(SqlAccessProfile::KnowledgeBase),
        AgentType::General => Ok(SqlAccessProfile::General),
        AgentType::SystemAdmin => Err("execute_query is not available to SystemAdmin".to_string()),
    }
}

fn parse_args<T: for<'de> Deserialize<'de>>(arguments: &str) -> Result<T, String> {
    serde_json::from_str(arguments).map_err(|error| format!("Invalid tool arguments: {error}"))
}

async fn save_query_artifact(
    args: &ExecuteQueryArgs,
    chat_id: &str,
    agent_id: &str,
    row_count: usize,
) -> Option<String> {
    let dto = crate::domain::a019_llm_artifact::service::LlmArtifactDto {
        id: None,
        code: None,
        description: args.description.clone(),
        comment: Some(format!("Возвращено строк: {row_count}")),
        chat_id: chat_id.to_string(),
        agent_id: agent_id.to_string(),
        artifact_type: Some("sql_query".to_string()),
        sql_query: args.sql.clone(),
        query_params: Some(json!({ "params": args.params, "limit": args.limit }).to_string()),
        visualization_config: None,
    };
    crate::domain::a019_llm_artifact::service::create(dto)
        .await
        .ok()
        .map(|id| id.to_string())
}

pub async fn execute_data_tool(
    name: &str,
    arguments: &str,
    agent_type: &AgentType,
    chat_id: &str,
    agent_id: &str,
) -> Value {
    let result: Result<Value, String> = match name {
        "list_data_sources" => parse_args::<ListSourcesArgs>(arguments)
            .map(|args| list_sources(args.kind))
            .and_then(|sources| {
                serde_json::to_value(json!({
                    "total": sources.len(),
                    "sources": sources,
                    "selection_order": ["dataview", "base", "raw"]
                }))
                .map_err(|error| error.to_string())
            }),
        "query_data_schema" => match parse_args::<SchemaQueryRequest>(arguments) {
            Ok(request) => query_schema(request)
                .await
                .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string())),
            Err(error) => Err(error),
        },
        "run_data_view_scalar" => match parse_args::<DataViewScalarRequest>(arguments) {
            Ok(request) => run_data_view_scalar(request)
                .await
                .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string())),
            Err(error) => Err(error),
        },
        "run_data_view_drilldown" => match parse_args::<DataViewDrilldownRequest>(arguments) {
            Ok(request) => run_data_view_drilldown(request)
                .await
                .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string())),
            Err(error) => Err(error),
        },
        "execute_query" => match parse_args::<ExecuteQueryArgs>(arguments) {
            Ok(args) => match access_profile(agent_type) {
                Ok(profile) => match execute_raw_query(
                    RawQueryRequest {
                        sql: args.sql.clone(),
                        params: args.params.clone(),
                        limit: args.limit,
                    },
                    profile,
                )
                .await
                {
                    Ok(tabular) => {
                        let artifact_id =
                            save_query_artifact(&args, chat_id, agent_id, tabular.row_count).await;
                        serde_json::to_value(tabular)
                            .map(|mut value| {
                                if let Some(id) = artifact_id {
                                    value["artifact_id"] = Value::String(id);
                                }
                                value
                            })
                            .map_err(|error| error.to_string())
                    }
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        },
        _ => Err(format!("Unknown data tool: {name}")),
    };
    result.unwrap_or_else(|error| json!({ "error": error }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn definitions_are_unique_and_include_typed_queries() {
        let definitions = tool_definitions();
        let names: HashSet<_> = definitions.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(names.len(), definitions.len());
        assert!(names.contains("list_data_sources"));
        assert!(names.contains("query_data_schema"));
        assert!(names.contains("run_data_view_scalar"));
        assert!(names.contains("run_data_view_drilldown"));
    }

    #[test]
    fn legacy_catalog_tools_are_gone() {
        // list_data_views / list_data_schemas удалены в пользу единого list_data_sources.
        let names: HashSet<_> = tool_definitions()
            .iter()
            .map(|tool| tool.name.clone())
            .collect();
        assert!(names.contains("list_data_sources"));
        assert!(!names.contains("list_data_views"));
        assert!(!names.contains("list_data_schemas"));
        assert!(!DATA_TOOL_NAMES.contains(&"list_data_views"));
        assert!(!DATA_TOOL_NAMES.contains(&"list_data_schemas"));
    }
}
