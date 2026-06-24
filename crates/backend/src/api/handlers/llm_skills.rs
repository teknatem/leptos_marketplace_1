//! Каталог LLM-навыков (skills) — read-only обзор реестра для UI.
//!
//! GET /api/llm-skills → { core_tools, skills: [{ id, title, description, intents, tools, allowed_for }], total }

use axum::Json;

/// Список навыков из реестра `shared/llm/skills.rs`.
pub async fn list() -> Json<serde_json::Value> {
    Json(crate::shared::llm::skills::catalog())
}
