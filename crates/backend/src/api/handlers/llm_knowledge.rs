use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::shared::llm::knowledge_base::{KnowledgeDoc, KNOWLEDGE_BASE};

#[derive(Debug, Deserialize)]
pub struct LlmKnowledgeListParams {
    #[serde(default)]
    pub tag: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LlmKnowledgeListItem {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub source_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LlmKnowledgeDetailResponse {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub source_path: Option<String>,
    pub content: String,
}

pub async fn list(Query(params): Query<LlmKnowledgeListParams>) -> Json<Vec<LlmKnowledgeListItem>> {
    let docs: Vec<&KnowledgeDoc> = if params.tag.is_empty() {
        KNOWLEDGE_BASE.all_docs()
    } else {
        let tags: Vec<&str> = params.tag.iter().map(String::as_str).collect();
        KNOWLEDGE_BASE.search_by_tags(&tags)
    };

    let mut items: Vec<LlmKnowledgeListItem> = docs
        .into_iter()
        .map(|doc| LlmKnowledgeListItem {
            id: doc.id.clone(),
            title: doc.title.clone(),
            tags: doc.tags.clone(),
            related: doc.related.clone(),
            source_path: doc.source_path.clone(),
        })
        .collect();

    items.sort_by(|a, b| a.id.cmp(&b.id));
    Json(items)
}

pub async fn get_by_id(
    Path(id): Path<String>,
) -> Result<Json<LlmKnowledgeDetailResponse>, StatusCode> {
    let Some(doc) = KNOWLEDGE_BASE.get(&id) else {
        return Err(StatusCode::NOT_FOUND);
    };

    Ok(Json(LlmKnowledgeDetailResponse {
        id: doc.id.clone(),
        title: doc.title.clone(),
        tags: doc.tags.clone(),
        related: doc.related.clone(),
        source_path: doc.source_path.clone(),
        content: doc.content.clone(),
    }))
}
