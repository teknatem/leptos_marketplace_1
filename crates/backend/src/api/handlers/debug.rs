//! Debug endpoints — только для разработки.
//!
//! GET  /api/debug/tool-test?tool=get_entity_schema&entity_index=a006
//! GET  /api/debug/tool-test?tool=list_entities&category=ref
//! GET  /api/debug/tool-test?tool=execute_query&sql=SELECT+1&description=test

use crate::shared::llm::types::ToolCall;
use axum::{extract::Query, response::IntoResponse, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ToolTestParams {
    /// Имя инструмента: list_entities / get_entity_schema / execute_query / ...
    pub tool: String,
    // Параметры конкретных инструментов
    pub entity_index: Option<String>,
    pub category: Option<String>,
    pub sql: Option<String>,
    pub description: Option<String>,
    pub from_entity: Option<String>,
    pub to_entity: Option<String>,
    pub tags: Option<String>,
    pub id: Option<String>,
}

/// GET /api/debug/tool-test
///
/// Напрямую вызывает execute_tool_call без участия LLM.
/// Позволяет убедиться что инструмент работает и что именно он возвращает.
pub async fn tool_test(Query(params): Query<ToolTestParams>) -> impl IntoResponse {
    // Собираем аргументы в JSON в зависимости от инструмента
    let args = build_args(&params);

    let call = ToolCall {
        id: "debug-test".to_string(),
        name: params.tool.clone(),
        arguments: args.to_string(),
    };

    tracing::info!(
        "[debug/tool-test] tool='{}' args={}",
        params.tool,
        call.arguments
    );

    let raw_result =
        crate::shared::llm::execute_tool_call(&call, "debug-chat-id", "debug-agent-id").await;

    tracing::info!(
        "[debug/tool-test] result preview: {}",
        &raw_result[..raw_result.len().min(500)]
    );

    let parsed: serde_json::Value = serde_json::from_str(&raw_result)
        .unwrap_or_else(|e| serde_json::json!({ "raw": raw_result, "parse_error": e.to_string() }));

    Json(serde_json::json!({
        "tool":   params.tool,
        "args":   serde_json::from_str::<serde_json::Value>(&call.arguments).ok(),
        "result": parsed,
        "ok":     parsed.get("error").is_none(),
    }))
}

fn build_args(p: &ToolTestParams) -> serde_json::Value {
    match p.tool.as_str() {
        "get_entity_schema" => serde_json::json!({
            "entity_index": p.entity_index.as_deref().unwrap_or("a006")
        }),
        "list_entities" => serde_json::json!({
            "category": p.category
        }),
        "execute_query" => serde_json::json!({
            "sql": p.sql.as_deref().unwrap_or("SELECT 1"),
            "description": p.description.as_deref().unwrap_or("debug test")
        }),
        "get_join_hint" => serde_json::json!({
            "from_entity": p.from_entity.as_deref().unwrap_or("a012"),
            "to_entity":   p.to_entity.as_deref().unwrap_or("a006")
        }),
        "search_knowledge" => serde_json::json!({
            "tags": p.tags.as_ref().map(|t| t.split(',').collect::<Vec<_>>()).unwrap_or_default()
        }),
        "get_knowledge" => serde_json::json!({
            "id": p.id.as_deref().unwrap_or("")
        }),
        "list_data_views" => serde_json::json!({}),
        _ => serde_json::json!({}),
    }
}
