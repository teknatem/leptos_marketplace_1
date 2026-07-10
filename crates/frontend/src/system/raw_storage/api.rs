use crate::shared::api_utils::api_base;
use crate::system::auth::storage;
use contracts::system::raw_storage::{
    DbVacuumResult, DbVacuumStatus, RawStorageCleanupPreview, RawStorageCleanupRequest,
    RawStorageSettings, RawStorageStatus,
};
use gloo_net::http::Request;

fn auth_header() -> Result<String, String> {
    storage::get_access_token()
        .map(|token| format!("Bearer {}", token))
        .ok_or_else(|| "Not authenticated".to_string())
}

pub async fn fetch_status() -> Result<RawStorageStatus, String> {
    let response = Request::get(&format!("{}/api/sys/raw-storage/status", api_base()))
        .header("Authorization", &auth_header()?)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch raw storage status: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch raw storage status: HTTP {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse raw storage status: {}", e))
}

pub async fn update_settings(capture_enabled: bool) -> Result<RawStorageSettings, String> {
    let dto = RawStorageSettings { capture_enabled };
    let response = Request::post(&format!("{}/api/sys/raw-storage/settings", api_base()))
        .header("Authorization", &auth_header()?)
        .json(&dto)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to update raw storage settings: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to update raw storage settings: HTTP {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse raw storage settings: {}", e))
}

pub async fn cleanup_preview(
    req: &RawStorageCleanupRequest,
) -> Result<RawStorageCleanupPreview, String> {
    let response = Request::post(&format!(
        "{}/api/sys/raw-storage/cleanup/preview",
        api_base()
    ))
    .header("Authorization", &auth_header()?)
    .json(req)
    .map_err(|e| format!("Failed to serialize cleanup request: {}", e))?
    .send()
    .await
    .map_err(|e| format!("Failed to preview raw storage cleanup: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to preview raw storage cleanup: HTTP {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse cleanup preview: {}", e))
}

pub async fn cleanup(req: &RawStorageCleanupRequest) -> Result<RawStorageCleanupPreview, String> {
    let response = Request::post(&format!("{}/api/sys/raw-storage/cleanup", api_base()))
        .header("Authorization", &auth_header()?)
        .json(req)
        .map_err(|e| format!("Failed to serialize cleanup request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Failed to cleanup raw storage: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to cleanup raw storage: HTTP {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse cleanup result: {}", e))
}

pub async fn fetch_vacuum_status() -> Result<DbVacuumStatus, String> {
    let response = Request::get(&format!("{}/api/sys/raw-storage/vacuum", api_base()))
        .header("Authorization", &auth_header()?)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch vacuum status: {}", e))?;

    if !response.ok() {
        return Err(format!(
            "Failed to fetch vacuum status: HTTP {}",
            response.status()
        ));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse vacuum status: {}", e))
}

pub async fn run_vacuum() -> Result<DbVacuumResult, String> {
    let response = Request::post(&format!("{}/api/sys/raw-storage/vacuum", api_base()))
        .header("Authorization", &auth_header()?)
        .send()
        .await
        .map_err(|e| format!("Failed to run VACUUM: {}", e))?;

    if !response.ok() {
        return Err(format!("Failed to run VACUUM: HTTP {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Failed to parse vacuum result: {}", e))
}
