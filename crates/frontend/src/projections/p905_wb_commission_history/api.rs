use contracts::projections::p905_wb_commission_history::dto::{
    CommissionDeleteResponse, CommissionHistoryDto, CommissionListResponse, CommissionSaveRequest,
    CommissionSaveResponse, CommissionSyncResponse,
};
use gloo_net::http::Request;

const API_BASE: &str = "/api/p905-commission";

/// Получить список комиссий с фильтрами
pub async fn list_commissions(
    date_from: Option<String>,
    date_to: Option<String>,
    subject_id: Option<i32>,
    sort_by: Option<String>,
    sort_desc: Option<bool>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<CommissionListResponse, String> {
    let mut url = format!("{}/list?", API_BASE);

    if let Some(from) = date_from {
        url.push_str(&format!("date_from={}&", from));
    }
    if let Some(to) = date_to {
        url.push_str(&format!("date_to={}&", to));
    }
    if let Some(sid) = subject_id {
        url.push_str(&format!("subject_id={}&", sid));
    }
    if let Some(sort) = sort_by {
        url.push_str(&format!("sort_by={}&", sort));
    }
    if let Some(desc) = sort_desc {
        url.push_str(&format!("sort_desc={}&", desc));
    }
    if let Some(lim) = limit {
        url.push_str(&format!("limit={}&", lim));
    }
    if let Some(off) = offset {
        url.push_str(&format!("offset={}&", off));
    }

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: CommissionListResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}

/// Получить комиссию по ID
pub async fn get_commission(id: &str) -> Result<CommissionHistoryDto, String> {
    let url = format!("{}/{}", API_BASE, id);

    let response = Request::get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: CommissionHistoryDto = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}

/// Создать или обновить комиссию
pub async fn save_commission(req: CommissionSaveRequest) -> Result<CommissionSaveResponse, String> {
    let url = API_BASE.to_string();

    let response = Request::post(&url)
        .json(&req)
        .map_err(|e| format!("Failed to serialize request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: CommissionSaveResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}

/// Удалить комиссию
pub async fn delete_commission(id: &str) -> Result<CommissionDeleteResponse, String> {
    let url = format!("{}/{}", API_BASE, id);

    let response = Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: CommissionDeleteResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}

/// Синхронизировать комиссии с API Wildberries
pub async fn sync_commissions() -> Result<CommissionSyncResponse, String> {
    let url = format!("{}/sync", API_BASE);

    let response = Request::post(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.ok() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let data: CommissionSyncResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(data)
}
