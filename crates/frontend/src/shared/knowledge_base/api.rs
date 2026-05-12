use crate::shared::api_utils::api_base;
use gloo_net::http::Request;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KbArticleSummary {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub source_path: Option<String>,
    pub display_path: String,
    pub is_embedded: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KbArticleDetail {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub source_path: Option<String>,
    pub display_path: String,
    pub is_embedded: bool,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KbStatsResponse {
    pub total_articles: usize,
    pub file_articles: usize,
    pub embedded_articles: usize,
    pub total_tags: usize,
    pub total_related: usize,
    pub total_folders: usize,
    pub knowledge_base_path: String,
    pub top_tags: Vec<KbCountItem>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KbCountItem {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KbTreeResponse {
    pub roots: Vec<KbTreeNode>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KbTreeNode {
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub article: Option<KbArticleSummary>,
    pub children: Vec<KbTreeNode>,
}

pub async fn fetch_kb_stats() -> Result<KbStatsResponse, String> {
    fetch_json("/api/kb/stats").await
}

pub async fn fetch_kb_tree() -> Result<KbTreeResponse, String> {
    fetch_json("/api/kb/tree").await
}

pub async fn fetch_kb_article(id: &str) -> Result<KbArticleDetail, String> {
    fetch_json(&format!("/api/kb/articles/{}", urlencoding::encode(id))).await
}

async fn fetch_json<T>(path: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let url = format!("{}{}", api_base(), path);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Ошибка сети: {}", e))?;
    if !response.ok() {
        return Err(format!("Ошибка сервера: HTTP {}", response.status()));
    }
    response
        .json::<T>()
        .await
        .map_err(|e| format!("Ошибка парсинга: {}", e))
}
