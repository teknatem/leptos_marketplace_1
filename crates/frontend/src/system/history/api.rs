use contracts::system::history::{PageHistoryDto, PageHistoryRecordRequest};
use gloo_net::http::Request;

use crate::shared::api_utils::api_base;
use crate::system::auth::storage;

fn auth_header() -> Result<String, String> {
    storage::get_access_token()
        .map(|token| format!("Bearer {}", token))
        .ok_or_else(|| "Not authenticated".to_string())
}

pub async fn list_history(limit: Option<u64>) -> Result<Vec<PageHistoryDto>, String> {
    let url = match limit {
        Some(limit) => format!("{}/api/system/page-history?limit={}", api_base(), limit),
        None => format!("{}/api/system/page-history", api_base()),
    };
    let response = Request::get(&url)
        .header("Authorization", &auth_header()?)
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch page history: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to fetch page history: {}", response.status()));
    }

    response
        .json::<Vec<PageHistoryDto>>()
        .await
        .map_err(|e| format!("Failed to parse page history: {}", e))
}

pub async fn record(tab_key: &str, title: &str) -> Result<(), String> {
    let req = PageHistoryRecordRequest {
        tab_key: tab_key.to_string(),
        title: title.to_string(),
    };
    let response = Request::post(&format!("{}/api/system/page-history", api_base()))
        .header("Authorization", &auth_header()?)
        .json(&req)
        .map_err(|e| format!("Failed to serialize page history: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to record page history: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to record page history: {}", response.status()));
    }
    Ok(())
}

pub async fn clear_history() -> Result<(), String> {
    let response = Request::delete(&format!("{}/api/system/page-history", api_base()))
        .header("Authorization", &auth_header()?)
        .send()
        .await
        .map_err(|e| format!("Failed to clear page history: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to clear page history: {}", response.status()));
    }
    Ok(())
}
