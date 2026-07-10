//! Каталог LLM-инструментов (tools) — read-only обзор реестра для UI.
//!
//! GET /api/llm-tools → { tools: [{ name, description, parameters, category, is_core, skills }], total }

use axum::Json;

/// Полный каталог инструментов из «вселенной» `shared/llm/skills.rs`.
pub async fn list() -> Json<serde_json::Value> {
    Json(crate::shared::llm::skills::tools_catalog())
}
