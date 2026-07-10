use axum::{http::StatusCode, Json};
use contracts::system::raw_storage::{
    DbVacuumResult, DbVacuumStatus, RawStorageCleanupPreview, RawStorageCleanupRequest,
    RawStorageSettings, RawStorageStatus,
};

pub async fn get_status() -> Result<Json<RawStorageStatus>, StatusCode> {
    crate::shared::data::raw_storage::status()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get raw storage status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn set_settings(
    Json(dto): Json<RawStorageSettings>,
) -> Result<Json<RawStorageSettings>, StatusCode> {
    crate::system::settings::service::set_raw_json_capture_enabled(dto.capture_enabled)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update raw storage settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(dto))
}

pub async fn cleanup_preview(
    Json(req): Json<RawStorageCleanupRequest>,
) -> Result<Json<RawStorageCleanupPreview>, StatusCode> {
    crate::shared::data::raw_storage::cleanup_preview(&req)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to preview raw storage cleanup: {}", e);
            StatusCode::BAD_REQUEST
        })
}

pub async fn cleanup(
    Json(req): Json<RawStorageCleanupRequest>,
) -> Result<Json<RawStorageCleanupPreview>, StatusCode> {
    crate::shared::data::raw_storage::cleanup(&req)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to cleanup raw storage: {}", e);
            StatusCode::BAD_REQUEST
        })
}

pub async fn get_vacuum_status() -> Result<Json<DbVacuumStatus>, StatusCode> {
    crate::shared::data::raw_storage::vacuum_status()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get vacuum status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn run_vacuum() -> Result<Json<DbVacuumResult>, StatusCode> {
    crate::shared::data::raw_storage::vacuum()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to run VACUUM: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
